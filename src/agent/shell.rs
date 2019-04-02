use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};

use actix::{Recipient};

#[derive(Message)]
pub struct ShellOutput {
    pub id: String,
    pub bytes: Vec<u8>,
}

fn forward_child_output<T>(id: &String, mut child_output: T, recipient: Recipient<ShellOutput>) where T: Read + Send + 'static {
    let id = id.clone();
    std::thread::spawn(move || {
        let mut buf = [0; 1024];
        loop {
            match child_output.read(&mut buf) {
                Ok(n) => {
                    if n == 0 {
                        return;
                    }
                    if let Err(_) = recipient.do_send(ShellOutput{
                        id: id.clone(),
                        bytes: buf[0..n].to_vec(),
                    }) {
                        return;
                    }
                },
                Err(_) => {
                    return;
                },
            }
        }
    });
}

pub struct Shell {
    child: Child,
    child_stdin: ChildStdin,
}

impl Write for Shell {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.child_stdin.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Drop for Shell {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

impl Shell {
    pub fn new(id: &String, recipient: Recipient<ShellOutput>) -> std::io::Result<Shell> {
        match Command::new("sh")
            .args(vec!["-i"])
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn() {
                Ok(mut cmd) => {
                    let mut stderr = cmd.stderr.take().unwrap();
                    forward_child_output(&id, stderr, recipient.clone());

                    let mut stdout = cmd.stdout.take().unwrap();
                    forward_child_output(&id, stdout, recipient.clone());

                    Ok(Shell{
                        child_stdin: cmd.stdin.take().unwrap(),
                        child: cmd,
                    })
                },
                Err(e) => Err(e),
            }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Write};
    use std::time::{Duration, Instant};
    use actix::{Actor, Arbiter, Context, Handler, Message, System};
    use futures::{Future};
    use tokio_timer::{Delay};

    struct ShellTest {
        output: String,
    }

    impl Actor for ShellTest {
        type Context = Context<Self>;
    }

    impl Handler<super::ShellOutput> for ShellTest {
        type Result = ();

        fn handle(&mut self, msg: super::ShellOutput, _ctx: &mut Self::Context) {
            self.output.push_str(&String::from_utf8(msg.bytes).unwrap()); 
        }
    }

    struct GetOutput;

    impl Message for GetOutput {
        type Result = String;
    }

    impl Handler<GetOutput> for ShellTest {
        type Result = String;

        fn handle(&mut self, _msg: GetOutput, _ctx: &mut Self::Context) -> Self::Result {
            self.output.clone()
        }
    }

    impl ShellTest {
        fn new() -> ShellTest {
            ShellTest{
                output: String::new(),
            }
        }
    }

    #[test]
    fn shell() {
        System::run(|| {
            let addr = ShellTest::new().start();
            let mut shell = super::Shell::new(&"foo".to_string(), addr.clone().recipient()).unwrap();
            shell.write_all(b"echo bar\n").unwrap();

            Arbiter::spawn(
                Delay::new(Instant::now() + Duration::from_millis(500))
                    .then(move |_| addr.send(GetOutput{}))
                    .then(move |output| {
                        drop(shell);
                        assert!(output.unwrap().contains("bar"));
                        System::current().stop();
                        futures::future::result(Ok(()))
                    })
            );
        });
    }
}
