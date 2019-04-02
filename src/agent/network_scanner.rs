use std::collections::{HashMap};
use std::io::{ErrorKind};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use common;
use super::hyperdeck;

use futures::{future, Future, Async, Poll};
use futures::task::{current, Task};
use simple_error::{SimpleError};
use actix::{Actor, ActorFuture, AsyncContext, Context, Recipient};
use libc::{IFF_POINTOPOINT};
use ipnetwork::{IpNetwork};
use pnet::datalink::{self, Channel, Config, NetworkInterface, MacAddr};
use pnet::packet::ethernet::MutableEthernetPacket;
use pnet::packet::arp::MutableArpPacket;
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::{Packet, MutablePacket};
use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, ArpOperation, ArpPacket};

const SCAN_INTERVAL: Duration = Duration::from_secs(15);
const SCAN_TIMEOUT: Duration = Duration::from_secs(8);
const DEVICE_TIMEOUT: Duration = Duration::from_secs(60);
const PROBE_INTERVAL: Duration = Duration::from_secs(60);

struct Device {
    ip_address: Ipv4Addr,
    last_seen_time: Instant,
    last_probe_time: Instant,
    details: Option<common::NetworkDeviceDetails>,
}

#[derive(Message)]
pub struct NetworkState {
    pub devices: Vec<common::NetworkDevice>,
}

pub struct NetworkScanner {
    devices: HashMap<MacAddr, Device>,
    logger: slog::Logger,
    recipient: Recipient<NetworkState>,
}

impl Actor for NetworkScanner {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        info!(self.logger, "started");
        self.scan(ctx);
        ctx.run_interval(SCAN_INTERVAL, |act, ctx| {
            act.scan(ctx);
        });
    }
}

/// Probes the given device for details. Errors such as timeouts and protocol errors are highly
/// expected during black box probing, so the returned error should generally only be used for
/// debugging.
fn probe_device(mac_address: MacAddr, ip_address: Ipv4Addr) -> Box<Future<Item=common::NetworkDeviceDetails, Error=SimpleError>> {
    if mac_address.0 == 0x7c && mac_address.1 == 0x2e && mac_address.2 == 0x0d {
        let addr = SocketAddr::new(IpAddr::V4(ip_address), hyperdeck::DEFAULT_PORT);
        return Box::new(hyperdeck::HyperDeck::connect(&addr)
            .and_then(|hyperdeck| hyperdeck.write_command("device info".to_string()))
            .and_then(|hyperdeck| hyperdeck.read_command_response())
            .and_then(|(_, response)| {
                if response.code < 200 || response.code >= 300 {
                    return future::err(SimpleError::new(format!("unexpected response code: {}", response.code)));
                }
                let params = response.parse_payload_parameters();
                if let Err(e) = params {
                    return future::err(SimpleError::with("error parsing payload parameters", e));
                }
                let params = params.unwrap();
                future::ok(common::NetworkDeviceDetails::HyperDeckDetails(
                    common::HyperDeckDetails{
                        model_name: params.get("model").unwrap_or(&"").to_string(),
                        protocol_version: params.get("protocol version").unwrap_or(&"").to_string(),
                        unique_id: params.get("unique id").unwrap_or(&"").to_string(),
                    }
                ))
            })
        );
    }
    Box::new(future::err(SimpleError::new("unrecognized vendor")))
}

impl NetworkScanner {
    pub fn new(logger: slog::Logger, recipient: Recipient<NetworkState>) -> Self {
        Self {
            devices: HashMap::new(),
            logger: logger,
            recipient: recipient,
        }
    }

    fn scan(&self, ctx: &mut Context<Self>) {
        ctx.spawn(
            actix::fut::wrap_future::<_, Self>(scan_networks(SCAN_TIMEOUT))
            .then(|result, act, ctx| {
                match result {
                    Ok(devices) => {
                        let now = Instant::now();
                        for (mac_address, ip_address) in devices {
                            let v = act.devices.entry(mac_address).or_insert(Device{
                                ip_address: ip_address,
                                last_seen_time: now,
                                last_probe_time: now - Duration::from_secs(60 * 68),
                                details: None,
                            });
                            v.ip_address = ip_address;
                            v.last_seen_time = now;
                            if now.duration_since(v.last_probe_time) > PROBE_INTERVAL {
                                v.last_probe_time = now;
                                ctx.spawn(
                                    actix::fut::wrap_future::<_, Self>(probe_device(mac_address, ip_address))
                                        .map(move |details, act, _ctx| {
                                            if let Some(device) = act.devices.get_mut(&mac_address) {
                                                device.details = Some(details);
                                            }
                                        })
                                        .map_err(|_, _, _| ())
                                );
                            }
                        }
                        act.devices.retain(|_, device| now.duration_since(device.last_seen_time) < DEVICE_TIMEOUT);

                        let state = NetworkState{
                            devices: act.devices.iter()
                                .map(|(mac_address, device)| common::NetworkDevice{
                                    ip_address: format!("{}", device.ip_address),
                                    mac_address: format!("{}", mac_address),
                                    details: device.details.clone(),
                                })
                                .collect(),
                        };
                        if let Err(e) = act.recipient.do_send(state) {
                            error!(act.logger, "error sending state: {}", e);
                        }
                    },
                    Err(e) => {
                        error!(act.logger, "error scanning networks: {}", e);
                    }
                }
                actix::fut::ok(())
            })
        );
    }
}

struct ScanNetworkFutureStatus {
    result: Option<Result<HashMap<MacAddr, Ipv4Addr>, SimpleError>>,
    task: Option<Task>,
}

struct ScanNetworkFuture {
    status: Arc<Mutex<ScanNetworkFutureStatus>>,
}

impl Future for ScanNetworkFuture {
    type Item = HashMap<MacAddr, Ipv4Addr>;
    type Error = SimpleError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut r = self.status.lock().unwrap();
        if let Some(result) = r.result.clone() {
            return match result {
                Ok(devices) => Ok(Async::Ready(devices)),
                Err(e) => Err(e),
            }
        }
        r.task = Some(current());
        Ok(Async::NotReady)
    }
}

fn scan_network(interface: NetworkInterface, timeout: Duration) -> ScanNetworkFuture {
    let networks: Vec<&IpNetwork> = interface.ips.iter().filter(|ip| ip.is_ipv4() && ip.prefix() >= 16).collect();
    if networks.len() == 0 {
        return ScanNetworkFuture{
            status: Arc::new(Mutex::new(ScanNetworkFutureStatus{
                result: Some(Ok(HashMap::new())),
                task: None,
            })),
        };
    }

    let mut config = Config::default();
    config.read_timeout = Some(Duration::from_millis(200));
    let (mut tx, mut rx) = match datalink::channel(&interface, config) {
        Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => return ScanNetworkFuture{
            status: Arc::new(Mutex::new(ScanNetworkFutureStatus{
                result: Some(Err(SimpleError::new("unknown channel type"))),
                task: None,
            })),
        },
        Err(e) => return ScanNetworkFuture{
            status: Arc::new(Mutex::new(ScanNetworkFutureStatus{
                result: Some(Err(SimpleError::with("error creating channel", e))),
                task: None,
            })),
        },
    };

    let run = Arc::new(AtomicBool::new(true));
    let receiver_run = run.clone();

    let status = Arc::new(Mutex::new(ScanNetworkFutureStatus{
        result: None,
        task: None,
    }));
    let ret = ScanNetworkFuture{
        status: status.clone(),
    };

    thread::spawn(move || {
        let mut devices = HashMap::new();
        while receiver_run.load(Ordering::Relaxed) {
            match rx.next() {
                Ok(data) => {
                    let ethernet_packet = EthernetPacket::new(data).unwrap();
                    let ethernet_payload = ethernet_packet.payload();
                    let arp_packet = ArpPacket::new(ethernet_payload).unwrap();
                    let arp_reply_op = ArpOperation::new(2_u16);

                    if arp_packet.get_operation() == arp_reply_op {
                        devices.insert(arp_packet.get_sender_hw_addr(), arp_packet.get_sender_proto_addr());
                    }
                },
                Err(e) => {
                    if e.kind() != ErrorKind::TimedOut {
                        let mut w = status.lock().unwrap();
                        w.result = Some(Err(SimpleError::with("error receiving packet", e)));
                        if let Some(ref task) = w.task {
                            task.notify();
                        }
                        return;
                    }
                }
            }
        }

        let mut w = status.lock().unwrap();
        w.result = Some(Ok(devices));
        if let Some(ref task) = w.task {
            task.notify();
        }
    });

    let source_mac = interface.mac_address();
    let target_mac = MacAddr::new(255,255,255,255,255,255);

    for network in networks {
        if let IpNetwork::V4(network) = network {
            for ip in network.iter() {
                let mut ethernet_buffer = [0u8; 42];
                let mut ethernet_packet = MutableEthernetPacket::new(&mut ethernet_buffer).unwrap();

                ethernet_packet.set_destination(target_mac);
                ethernet_packet.set_source(source_mac);
                ethernet_packet.set_ethertype(EtherTypes::Arp);

                let mut arp_buffer = [0u8; 28];
                let mut arp_packet = MutableArpPacket::new(&mut arp_buffer).unwrap();

                arp_packet.set_hardware_type(ArpHardwareTypes::Ethernet);
                arp_packet.set_protocol_type(EtherTypes::Ipv4);
                arp_packet.set_hw_addr_len(6);
                arp_packet.set_proto_addr_len(4);
                arp_packet.set_operation(ArpOperations::Request);
                arp_packet.set_sender_hw_addr(source_mac);
                arp_packet.set_sender_proto_addr(network.ip());
                arp_packet.set_target_hw_addr(target_mac);
                arp_packet.set_target_proto_addr(ip);

                ethernet_packet.set_payload(arp_packet.packet_mut());

                tx.send_to(ethernet_packet.packet(), Some(interface.clone()));
            }
        }
    }

    thread::spawn(move || {
        thread::sleep(timeout);
        run.store(false, Ordering::Relaxed);
    });

    ret
}

fn scan_networks(timeout: Duration) -> impl Future<Item=HashMap<MacAddr, Ipv4Addr>, Error=SimpleError> {
    let interfaces = datalink::interfaces().into_iter().filter(|iface| !iface.ips.is_empty() && !iface.is_loopback() && iface.flags & (IFF_POINTOPOINT as u32) == 0);
    future::join_all(interfaces.map(move |iface| scan_network(iface, timeout)))
        .and_then(|vecs| {
            future::ok(vecs.into_iter().flatten().collect())
        })
}

#[cfg(test)]
mod tests {
    use std::time::{Duration};
    use actix::{Arbiter, System};
    use futures::{future, Future};

    #[test]
    fn scan_networks() {
        System::run(|| {
            Arbiter::spawn(
                super::scan_networks(Duration::from_secs(1))
                    .then(|result| {
                        assert!(result.unwrap().len() > 0);
                        System::current().stop();
                        future::result(Ok(()))
                    })
            );
        });
    }
}
