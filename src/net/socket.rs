use socket2::{Domain, Protocol, SockAddr, Socket as Socket2, Type};
use std::{io, net::Shutdown};

#[cfg(feature = "runtime")]
use crate::{
    buf::{IntoInner, IoBuf, IoBufMut},
    driver::AsRawFd,
    op::{
        Accept, BufResultExt, Connect, Recv, RecvFrom, RecvFromVectored, RecvResultExt,
        RecvVectored, Send, SendTo, SendToVectored, SendVectored,
    },
    task::RUNTIME,
    BufResult,
};

pub struct Socket {
    pub(crate) socket: Socket2,
}

impl Socket {
    pub fn from_socket2(socket: Socket2) -> io::Result<Self> {
        let this = Self { socket };
        #[cfg(feature = "runtime")]
        RUNTIME.with(|runtime| runtime.attach(this.as_raw_fd()))?;
        Ok(this)
    }

    pub fn as_socket2(&self) -> &Socket2 {
        &self.socket
    }

    pub fn into_socket2(self) -> Socket2 {
        self.socket
    }

    pub fn peer_addr(&self) -> io::Result<SockAddr> {
        self.socket.peer_addr()
    }

    pub fn local_addr(&self) -> io::Result<SockAddr> {
        self.socket.local_addr()
    }

    #[allow(dead_code)]
    pub fn r#type(&self) -> io::Result<Type> {
        self.socket.r#type()
    }

    #[cfg(target_os = "linux")]
    #[allow(dead_code)]
    pub fn protocol(&self) -> io::Result<Option<Protocol>> {
        self.socket.protocol()
    }

    #[cfg(target_os = "windows")]
    #[allow(dead_code)]
    pub fn protocol(&self) -> io::Result<Option<Protocol>> {
        #[allow(unused_imports)]
        use crate::driver::AsRawFd;
        use windows_sys::Win32::Networking::WinSock::{
            getsockopt, SOL_SOCKET, SO_PROTOCOL_INFO, WSAPROTOCOL_INFOW,
        };

        let mut info: WSAPROTOCOL_INFOW = unsafe { std::mem::zeroed() };
        let mut info_len = std::mem::size_of_val(&info) as _;
        let res = unsafe {
            getsockopt(
                self.as_raw_fd() as _,
                SOL_SOCKET,
                SO_PROTOCOL_INFO,
                &mut info as *mut _ as *mut _,
                &mut info_len,
            )
        };
        if res != 0 {
            Err(io::Error::last_os_error())
        } else {
            match info.iProtocol {
                0 => Ok(None),
                p => Ok(Some(Protocol::from(p))),
            }
        }
    }

    pub fn new(domain: Domain, ty: Type, protocol: Option<Protocol>) -> io::Result<Self> {
        Self::from_socket2(Socket2::new(domain, ty, protocol)?)
    }

    pub fn bind(addr: &SockAddr, ty: Type, protocol: Option<Protocol>) -> io::Result<Self> {
        let socket = Self::new(addr.domain(), ty, protocol)?;
        socket.socket.bind(addr)?;
        Ok(socket)
    }

    pub fn listen(&self, backlog: i32) -> io::Result<()> {
        self.socket.listen(backlog)
    }

    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.socket.shutdown(how)
    }

    pub fn connect(&self, addr: &SockAddr) -> io::Result<()> {
        self.socket.connect(addr)
    }

    #[cfg(feature = "runtime")]
    pub async fn connect_async(&self, addr: &SockAddr) -> io::Result<()> {
        let op = Connect::new(self.as_raw_fd(), addr.clone());
        let (res, _) = RUNTIME.with(|runtime| runtime.submit(op)).await;
        res.map(|_| ())
    }

    #[cfg(all(feature = "runtime", target_os = "linux"))]
    pub async fn accept(&self) -> io::Result<(Self, SockAddr)> {
        use std::os::fd::FromRawFd;

        let op = Accept::new(self.as_raw_fd());
        let (res, op) = RUNTIME.with(|runtime| runtime.submit(op)).await;
        let accept_sock = Self::from_socket2(unsafe { Socket2::from_raw_fd(res? as _) })?;
        let addr = op.into_addr();
        Ok((accept_sock, addr))
    }

    #[cfg(all(feature = "runtime", target_os = "windows"))]
    pub async fn accept(&self) -> io::Result<(Self, SockAddr)> {
        let local_addr = self.local_addr()?;
        let accept_sock = Self::new(local_addr.domain(), self.r#type()?, self.protocol()?)?;
        let op = Accept::new(self.as_raw_fd(), accept_sock.as_raw_fd() as _);
        let (res, op) = RUNTIME.with(|runtime| runtime.submit(op)).await;
        res?;
        let addr = op.into_addr()?;
        Ok((accept_sock, addr))
    }

    #[cfg(feature = "runtime")]
    pub async fn recv<T: IoBufMut>(&self, buffer: T) -> BufResult<usize, T> {
        let op = Recv::new(self.as_raw_fd(), buffer);
        RUNTIME
            .with(|runtime| runtime.submit(op))
            .await
            .into_inner()
            .map_advanced()
            .into_inner()
    }

    #[cfg(feature = "runtime")]
    pub async fn recv_vectored<T: IoBufMut>(&self, buffer: Vec<T>) -> BufResult<usize, Vec<T>> {
        let op = RecvVectored::new(self.as_raw_fd(), buffer);
        RUNTIME
            .with(|runtime| runtime.submit(op))
            .await
            .into_inner()
            .map_advanced()
            .into_inner()
    }

    #[cfg(feature = "runtime")]
    pub async fn send<T: IoBuf>(&self, buffer: T) -> BufResult<usize, T> {
        let op = Send::new(self.as_raw_fd(), buffer);
        RUNTIME
            .with(|runtime| runtime.submit(op))
            .await
            .into_inner()
            .into_inner()
    }

    #[cfg(feature = "runtime")]
    pub async fn send_vectored<T: IoBuf>(&self, buffer: Vec<T>) -> BufResult<usize, Vec<T>> {
        let op = SendVectored::new(self.as_raw_fd(), buffer);
        RUNTIME
            .with(|runtime| runtime.submit(op))
            .await
            .into_inner()
            .into_inner()
    }

    #[cfg(feature = "runtime")]
    pub async fn recv_from<T: IoBufMut>(&self, buffer: T) -> BufResult<(usize, SockAddr), T> {
        let op = RecvFrom::new(self.as_raw_fd(), buffer);
        RUNTIME
            .with(|runtime| runtime.submit(op))
            .await
            .into_inner()
            .map_addr()
            .map_advanced()
            .into_inner()
    }

    #[cfg(feature = "runtime")]
    pub async fn recv_from_vectored<T: IoBufMut>(
        &self,
        buffer: Vec<T>,
    ) -> BufResult<(usize, SockAddr), Vec<T>> {
        let op = RecvFromVectored::new(self.as_raw_fd(), buffer);
        RUNTIME
            .with(|runtime| runtime.submit(op))
            .await
            .into_inner()
            .map_addr()
            .map_advanced()
            .into_inner()
    }

    #[cfg(feature = "runtime")]
    pub async fn send_to<T: IoBuf>(&self, buffer: T, addr: &SockAddr) -> BufResult<usize, T> {
        let op = SendTo::new(self.as_raw_fd(), buffer, addr.clone());
        RUNTIME
            .with(|runtime| runtime.submit(op))
            .await
            .into_inner()
            .into_inner()
    }

    #[cfg(feature = "runtime")]
    pub async fn send_to_vectored<T: IoBuf>(
        &self,
        buffer: Vec<T>,
        addr: &SockAddr,
    ) -> BufResult<usize, Vec<T>> {
        let op = SendToVectored::new(self.as_raw_fd(), buffer, addr.clone());
        RUNTIME
            .with(|runtime| runtime.submit(op))
            .await
            .into_inner()
            .into_inner()
    }
}
