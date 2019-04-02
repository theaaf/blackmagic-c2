use common;

use std::time::{Duration};

use actix::{Actor, AsyncContext, Context, Recipient};

const POLL_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Message)]
pub struct DeckLinkState {
    pub devices: Vec<common::DeckLinkDevice>,
}

pub struct DeckLinkWatcher {
    logger: slog::Logger,
    recipient: Recipient<DeckLinkState>,
}

impl Actor for DeckLinkWatcher {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        info!(self.logger, "started");
        self.poll(ctx)
    }
}

fn collect_display_modes(iterator: decklink::DisplayModeIterator) -> Vec<common::DeckLinkDisplayMode> {
    iterator.map(|mode| common::DeckLinkDisplayMode{
        id: mode.get_id().0 as i32,
        name: mode.get_name().ok().unwrap_or("".to_string()),
    }).collect()
}

impl DeckLinkWatcher {
    pub fn new(logger: slog::Logger, recipient: Recipient<DeckLinkState>) -> Self {
        Self {
            logger: logger,
            recipient: recipient,
        }
    }

    fn poll(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(POLL_INTERVAL, |act, _ctx| {
            let iterator = match decklink::Iterator::new() {
                Ok(iterator) => iterator,
                Err(e) => {
                    error!(act.logger, "{}", e);
                    return;
                },
            };
            let mut devices = Vec::new();
            for device in iterator {
                let mut input = match device.query_input() {
                    Ok(input) => input,
                    Err(e) => {
                        error!(act.logger, "error getting device input: {:?}", e);
                        continue
                    },
                };

                let display_modes = match input.get_display_mode_iterator() {
                    Ok(iterator) => collect_display_modes(iterator),
                    Err(e) => {
                        error!(act.logger, "error getting device input modes: {:?}", e);
                        continue
                    },
                };

                let input = common::DeckLinkDeviceInput{
                    display_modes: display_modes,
                };

                let mut output = match device.query_output() {
                    Ok(output) => output,
                    Err(e) => {
                        error!(act.logger, "error getting device output: {:?}", e);
                        continue
                    },
                };

                let display_modes = match output.get_display_mode_iterator() {
                    Ok(iterator) => collect_display_modes(iterator),
                    Err(e) => {
                        error!(act.logger, "error getting device output modes: {:?}", e);
                        continue
                    },
                };

                let output = common::DeckLinkDeviceOutput{
                    display_modes: display_modes,
                };

                let attributes = match device.query_attributes() {
                    Ok(attributes) => attributes,
                    Err(e) => {
                        error!(act.logger, "error getting device attributes: {:?}", e);
                        continue
                    },
                };

                let attributes = common::DeckLinkDeviceAttributes{
                    supports_internal_keying: attributes.get_supports_internal_keying().ok(),
                    supports_external_keying: attributes.get_supports_external_keying().ok(),
                    supports_hd_keying: attributes.get_supports_hd_keying().ok(),
                    supports_input_format_detection: attributes.get_supports_input_format_detection().ok(),
                    has_reference_input: attributes.get_has_reference_input().ok(),
                    has_serial_port: attributes.get_has_serial_port().ok(),
                    has_analog_video_output_gain: attributes.get_has_analog_video_output_gain().ok(),
                    can_only_adjust_overall_video_output_gain: attributes.get_can_only_adjust_overall_video_output_gain().ok(),
                    has_video_input_antialiasing_filter: attributes.get_has_video_input_antialiasing_filter().ok(),
                    has_bypass: attributes.get_has_bypass().ok(),
                    supports_clock_timing_adjustment: attributes.get_supports_clock_timing_adjustment().ok(),
                    supports_full_duplex: attributes.get_supports_full_duplex().ok(),
                    supports_full_frame_reference_input_timing_offset: attributes.get_supports_full_frame_reference_input_timing_offset().ok(),
                    supports_smpte_level_a_output: attributes.get_supports_smpte_level_a_output().ok(),
                    supports_dual_link_sdi: attributes.get_supports_dual_link_sdi().ok(),
                    supports_quad_link_sdi: attributes.get_supports_quad_link_sdi().ok(),
                    supports_idle_output: attributes.get_supports_idle_output().ok(),
                    has_ltc_timecode_input: attributes.get_has_ltc_timecode_input().ok(),
                    supports_duplex_mode_configuration: attributes.get_supports_duplex_mode_configuration().ok(),
                    supports_hdr_metadata: attributes.get_supports_hdr_metadata().ok(),
                    supports_colorspace_metadata: attributes.get_supports_colorspace_metadata().ok(),
                    maximum_audio_channels: attributes.get_maximum_audio_channels().ok().map(|n| n as i32),
                    maximum_analog_audio_input_channels: attributes.get_maximum_analog_audio_input_channels().ok().map(|n| n as i32),
                    maximum_analog_audio_output_channels: attributes.get_maximum_analog_audio_output_channels().ok().map(|n| n as i32),
                    number_of_subdevices: attributes.get_number_of_subdevices().ok().map(|n| n as i32),
                    subdevice_index: attributes.get_subdevice_index().ok().map(|n| n as i32),
                    persistent_id: attributes.get_persistent_id().ok().map(|n| n as i32),
                    device_group_id: attributes.get_device_group_id().ok().map(|n| n as i32),
                    topological_id: attributes.get_topological_id().ok().map(|n| n as i32),
                    video_io_support: attributes.get_video_io_support().ok().map(|v| common::DeckLinkVideoIOSupport{
                        playback: v.contains(decklink::VideoIOSupport::PLAYBACK),
                        capture: v.contains(decklink::VideoIOSupport::CAPTURE),
                    }),
                    audio_input_rca_channel_count: attributes.get_audio_input_rca_channel_count().ok().map(|n| n as i32),
                    audio_input_xlr_channel_count: attributes.get_audio_input_xlr_channel_count().ok().map(|n| n as i32),
                    audio_output_rca_channel_count: attributes.get_audio_output_rca_channel_count().ok().map(|n| n as i32),
                    audio_output_xlr_channel_count: attributes.get_audio_output_xlr_channel_count().ok().map(|n| n as i32),
                    paired_device_persistent_id: attributes.get_paired_device_persistent_id().ok().map(|n| n as i32),
                    video_input_gain_minimum: attributes.get_video_input_gain_minimum().ok(),
                    video_input_gain_maximum: attributes.get_video_input_gain_maximum().ok(),
                    video_output_gain_minimum: attributes.get_video_output_gain_minimum().ok(),
                    video_output_gain_maximum: attributes.get_video_output_gain_maximum().ok(),
                    microphone_input_gain_minimum: attributes.get_microphone_input_gain_minimum().ok(),
                    microphone_input_gain_maximum: attributes.get_microphone_input_gain_maximum().ok(),
                    serial_port_device_name: attributes.get_serial_port_device_name().ok(),
                    vendor_name: attributes.get_vendor_name().ok(),
                    display_name: attributes.get_display_name().ok(),
                    model_name: attributes.get_model_name().ok(),
                    device_handle: attributes.get_device_handle().ok(),
                };

                let mut status = match device.query_status() {
                    Ok(status) => status,
                    Err(e) => {
                        error!(act.logger, "error getting device status: {:?}", e);
                        continue
                    },
                };

                let status = common::DeckLinkDeviceStatus{
                    video_input_signal_locked: status.get_video_input_signal_locked().ok(),
                    reference_signal_locked: status.get_reference_signal_locked().ok(),
                    received_edid: status.get_received_edid().ok(),
                    detected_video_input_mode: status.get_detected_video_input_mode().ok().and_then(|id| input.display_modes.iter().find(|m| m.id == id.0 as i32).cloned()),
                    current_video_input_mode: status.get_current_video_input_mode().ok().and_then(|id| input.display_modes.iter().find(|m| m.id == id.0 as i32).cloned()),
                    current_video_output_mode: status.get_current_video_output_mode().ok().and_then(|id| input.display_modes.iter().find(|m| m.id == id.0 as i32).cloned()),
                    pci_express_link_width: status.get_pci_express_link_width().ok().map(|n| n as i32),
                    pci_express_link_speed: status.get_pci_express_link_speed().ok().map(|n| n as i32),
                    busy: status.get_busy().ok().map(|v| common::DeckLinkDeviceBusyState{
                        playback_busy: v.contains(decklink::DeviceBusyState::PLAYBACK_BUSY),
                        capture_busy: v.contains(decklink::DeviceBusyState::CAPTURE_BUSY),
                        serial_port_busy: v.contains(decklink::DeviceBusyState::SERIAL_PORT_BUSY),
                    }),
                    device_temperature: status.get_device_temperature().ok().map(|n| n as i32),
                };

                devices.push(common::DeckLinkDevice{
                    model_name: device.get_model_name().ok().unwrap_or("".to_string()),
                    attributes: attributes,
                    input: input,
                    output: output,
                    status: status,
                });
            }
            let state = DeckLinkState{
                devices: devices,
            };
            if let Err(e) = act.recipient.do_send(state) {
                error!(act.logger, "{}", e);
            }
        });
    }
}
