use crate::protocol::Message;
use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};

pub struct Transport;

impl Transport {
    pub async fn send_tcp(stream: &mut TcpStream, message: &Message) -> Result<()> {
        let data = bincode::serialize(message)?;
        let len = data.len() as u32;
        stream.write_all(&len.to_be_bytes()).await?;
        stream.write_all(&data).await?;
        Ok(())
    }

    pub async fn recv_tcp(stream: &mut TcpStream) -> Result<Message> {
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        
        let mut data = vec![0u8; len];
        stream.read_exact(&mut data).await?;
        
        let message = bincode::deserialize(&data)?;
        Ok(message)
    }

    pub async fn send_udp(socket: &UdpSocket, addr: &str, message: &Message) -> Result<()> {
        let data = bincode::serialize(message)?;
        socket.send_to(&data, addr).await?;
        Ok(())
    }
}
