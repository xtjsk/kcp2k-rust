use bytes::Bytes;

#[derive(Debug, Clone, PartialEq)]
pub struct Packet {
    pub conv: u64,
    pub cmd: u8,
    pub data: Bytes,
}

impl Packet {
    pub fn new(conv: u64, cmd: u8, data: Bytes) -> Self {
        Packet { conv, cmd, data }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&self.conv.to_le_bytes()); // 使用 8 字节表示 u64
        buffer.push(self.cmd);
        buffer.extend_from_slice(&self.data);
        buffer
    }

    pub fn deserialize(buffer: Bytes) -> (u64, u8, Bytes) {
        if buffer.len() < 9 {
            return (0, 0, Bytes::new());
        }

        // 将前 8 个字节转换为 u64
        let conv = u64::from_le_bytes(buffer[0..8].try_into().unwrap());

        // 第 9 个字节是 cmd
        let cmd = buffer[8];

        // 剩下的字节是 data
        let data = buffer.slice(9..);

        (conv, cmd, data)
    }

    pub fn deserialize2struct(buffer: Bytes) -> Packet {
        let (conv, cmd, data) = Packet::deserialize(buffer);
        Packet { conv, cmd, data }
    }

    pub fn vec_u8_to_string(data: Bytes) -> String {
        String::from_utf8_lossy(data.iter().as_slice()).to_string()
    }
}
