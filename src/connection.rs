use std::{
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
};

use async_trait::async_trait;
use futures::future;
use tokio::{
    io::{split, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
};

pub trait ToVec {
    fn to_vec(&self) -> Vec<u8>;
}

// impl ToVec for P64 {
//     fn to_vec(&self) -> Vec<u8> {
//         self.0.to_le_bytes().to_vec()
//     }
// }

// impl ToVec for Payload {
//     fn to_vec(&self) -> Vec<u8> {
//         self.as_bytes().to_vec()
//     }
// }

impl ToVec for Vec<u8> {
    fn to_vec(&self) -> Vec<u8> {
        self.clone()
    }
}

impl<const N: usize> ToVec for [u8; N] {
    fn to_vec(&self) -> Vec<u8> {
        self[..].to_vec()
    }
}

impl ToVec for [u8] {
    fn to_vec(&self) -> Vec<u8> {
        self.to_vec()
    }
}

pub struct Socket {
    remote_addr: SocketAddr,
    reader: ReadHalf<TcpStream>,
    writer: WriteHalf<TcpStream>,
}

#[async_trait]
pub trait Connection: Sized {
    type Reader: Send + Unpin + AsyncRead;
    type Writer: Send + Unpin + AsyncWrite;

    fn reader_mut(&mut self) -> &mut Self::Reader;
    fn writer_mut(&mut self) -> &mut Self::Writer;
    fn reader_and_writer_mut(&mut self) -> (&mut Self::Reader, &mut Self::Writer);

    async fn send<D: ?Sized + ToVec + Sync>(&mut self, data: &D) -> io::Result<()> {
        let writer = self.writer_mut();
        writer.write_all(data.to_vec().as_slice()).await?;
        writer.flush().await?;
        Ok(())
    }

    async fn sendline<D: ?Sized + ToVec + Sync>(&mut self, data: &D) -> io::Result<()> {
        self.send(data).await?;
        self.send(b"\n").await?;
        Ok(())
    }

    async fn recvuntil(&mut self, pattern: &[u8]) -> io::Result<Vec<u8>> {
        let reader = self.reader_mut();
        let mut buf = vec![];
        loop {
            let mut buf_ = [0];
            reader.read_exact(&mut buf_).await?;
            buf.extend_from_slice(&buf_[..]);
            if buf.ends_with(pattern) {
                break;
            }
        }
        Ok(buf)
    }

    async fn recvline(&mut self) -> io::Result<Vec<u8>> {
        self.recvuntil(b"\n").await
    }

    async fn interactive(mut self) -> io::Result<()> {
        let (reader, writer) = self.reader_and_writer_mut();
        future::try_join(
            tokio::io::copy(&mut tokio::io::stdin(), writer),
            tokio::io::copy(reader, &mut tokio::io::stdout()),
        )
        .await?;
        Ok(())
    }
}

impl Socket {
    pub async fn new(port: u16) -> io::Result<Self> {
        let addr = Ipv4Addr::new(0, 0, 0, 0);
        let socket_addr = SocketAddrV4::new(addr, port);
        let listener = tokio::net::TcpListener::bind(socket_addr).await.unwrap();
        let (stream, remote_addr) = listener.accept().await.unwrap();
        let (reader, writer) = split(stream);

        Ok(Self {
            remote_addr,
            reader,
            writer,
        })
    }
}

impl Connection for Socket {
    type Reader = ReadHalf<TcpStream>;
    type Writer = WriteHalf<TcpStream>;

    fn reader_mut(&mut self) -> &mut Self::Reader {
        &mut self.reader
    }

    fn writer_mut(&mut self) -> &mut Self::Writer {
        &mut self.writer
    }

    fn reader_and_writer_mut(&mut self) -> (&mut Self::Reader, &mut Self::Writer) {
        (&mut self.reader, &mut self.writer)
    }
}
