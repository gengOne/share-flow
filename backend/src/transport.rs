use crate::protocol::Message;
use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};

pub struct Transport;

impl Transport {
    pub async fn send_tcp(stream: &mut TcpStream, message: &Message) -> Result<()> {
        let data = bincode::serialize(message)?;
        let len = data.len() as u32;
        
        // Coalesce writes: Create a single buffer with length prefix + data
        // This ensures the OS sends the packet immediately with TCP_NODELAY
        let mut buffer = Vec::with_capacity(4 + data.len());
        buffer.extend_from_slice(&len.to_be_bytes());
        buffer.extend_from_slice(&data);
        
        stream.write_all(&buffer).await?;
        stream.flush().await?; // 立即刷新缓冲区，确保数据立即发送
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

    // Split stream versions for concurrent read/write
    pub async fn send_tcp_split<W: AsyncWriteExt + Unpin>(writer: &mut W, message: &Message) -> Result<()> {
        let data = bincode::serialize(message)?;
        let len = data.len() as u32;
        
        let mut buffer = Vec::with_capacity(4 + data.len());
        buffer.extend_from_slice(&len.to_be_bytes());
        buffer.extend_from_slice(&data);
        
        writer.write_all(&buffer).await?;
        writer.flush().await?;
        Ok(())
    }

    pub async fn recv_tcp_split<R: AsyncReadExt + Unpin>(reader: &mut R) -> Result<Message> {
        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        
        let mut data = vec![0u8; len];
        reader.read_exact(&mut data).await?;
        
        let message = bincode::deserialize(&data)?;
        Ok(message)
    }
}
