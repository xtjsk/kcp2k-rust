// 定义一个枚举来封装不同的错误类型。
#[derive(Debug, Clone)]
pub enum ErrorCode {
    None,             // 无错误
    DnsResolve,       // 无法解析主机名
    Timeout,          // ping 超时或失效链接
    Congestion,       // 超出传输/网络可以处理的消息数
    InvalidReceive,   // RECV 无效数据包（可能是故意攻击）
    InvalidSend,      // 用户尝试发送无效数据
    ConnectionClosed, // 连接自愿关闭或非自愿丢失
    Unexpected,       // 意外错误/异常，需要修复。
    SendError,        // 发送数据失败
    ConnectionNotFound, // 未找到连接
}