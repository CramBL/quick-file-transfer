use std::{ffi::OsStr, net::ToSocketAddrs, time::Duration};

use ssh::{ExecBroker, SessionBroker, SshError, SshResult};

use super::remote_find_free_port::remote_find_free_port;

pub struct RemoteSshSession {
    session: SessionBroker,
    latest_executed_cmd: Option<ExecutedCmd>,
}

impl RemoteSshSession {
    pub fn new<A>(
        username: &str,
        private_key_path: &OsStr,
        addr: A,
        timeout: Option<Duration>,
    ) -> Result<Self, SshError>
    where
        A: ToSocketAddrs,
    {
        let passwd = super::util::get_remote_password_from_env();

        let session = ssh::create_session()
            .username(username)
            .password(passwd.as_deref().unwrap_or("root"))
            .private_key_path(private_key_path)
            .connect_with_timeout(addr, timeout)?;
        Ok(Self {
            session: session.run_backend(),
            latest_executed_cmd: None,
        })
    }

    pub fn find_free_port(&mut self, start_port: u16, end_port: u16) -> anyhow::Result<u16> {
        remote_find_free_port(&mut self.session, start_port, end_port)
    }

    fn open_run_exec(&mut self, cmd: &str) -> SshResult<ExecutedCmd> {
        let executed = ExecutedCmd::new(&mut self.session, cmd)?;
        Ok(executed)
    }

    /// Runs the remote command through SSH and returns the exit status of the command
    ///
    /// If you also want the output of the command, use `run_cmd_get_result`.
    pub fn run_cmd(&mut self, cmd: &str) -> SshResult<u32> {
        let executed = self.open_run_exec(cmd)?;
        let exit_status = executed.exit_status()?;
        log::trace!("Remote command exit status: {exit_status}");
        if let Some(terminate_msg) = executed.terminate_msg() {
            log::trace!("Remote command terminate message: {terminate_msg}");
        }

        self.latest_executed_cmd = Some(executed);

        Ok(exit_status)
    }

    /// Runs the remote command through SSH and returns the output/result of the command as UTF-8
    ///
    /// This will block until the server closes the channel (meaning the command has to run to end)
    pub fn run_cmd_get_result(&mut self, cmd: &str) -> SshResult<Vec<u8>> {
        let mut executed = self.open_run_exec(cmd)?;
        let exit_status = executed.exit_status()?;
        log::trace!("Remote command exit status: {exit_status}");
        if let Some(terminate_msg) = executed.terminate_msg() {
            log::trace!("Remote command terminate message: {terminate_msg}");
        }

        let res = executed.results()?;
        self.latest_executed_cmd = Some(executed);
        Ok(res)
    }

    /// If a command has been executed, consumes it and returns its result.
    ///
    /// Blocks until the server has closed the connection
    pub fn get_cmd_output(&mut self) -> Option<Vec<u8>> {
        let mut exec = self.latest_executed_cmd.take()?;
        exec.results().ok()
    }

    /// Close the remote session
    pub fn close(self) {
        self.session.close()
    }
}

pub struct ExecutedCmd {
    exec_broker: ExecBroker,
}

impl ExecutedCmd {
    pub fn new(session: &mut SessionBroker, cmd: &str) -> SshResult<Self> {
        let mut exec = session.open_exec()?;
        exec.send_command(cmd)?;
        Ok(Self { exec_broker: exec })
    }

    pub fn exit_status(&self) -> SshResult<u32> {
        self.exec_broker.exit_status()
    }

    pub fn terminate_msg(&self) -> Option<String> {
        let tm = self.exec_broker.terminate_msg().ok()?;
        if tm.is_empty() {
            None
        } else {
            Some(tm)
        }
    }

    /// Blocking
    pub fn results(&mut self) -> SshResult<Vec<u8>> {
        self.exec_broker.get_result()
    }
}
