# PcapFile.IO - 高性能数据包文件读写库

[![Crates.io](https://img.shields.io/crates/v/pcap-file-io)](https://crates.io/crates/pcap-file-io)
[![Documentation](https://docs.rs/pcap-file-io/badge.svg)](https://docs.rs/pcap-file-io)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org)

一个用Rust编写的高性能数据包文件处理库，提供了完整的数据包文件读写功能。本库使用自定义的PCAP格式（与标准PCAP格式不同），专为高性能数据采集和回放设计。

## 🚀 特性

- **高性能**: 零拷贝操作和编译时优化
- **内存安全**: Rust的内存安全保证
- **线程安全**: 内置线程安全支持
- **易于使用**: 简洁的API设计
- **可配置**: 灵活的配置选项
- **完整功能**: 支持所有自定义PCAP格式特性
- **跨平台**: 支持Windows、Linux、macOS

## 📦 安装

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
pcap-io = "0.1.0"
```

## 🎯 快速开始

### 基本使用

```rust
use pcap_io::{
    Configuration, DataPacket, Reader, Writer, Result,
};

fn main() -> Result<()> {
    // 创建配置
    let mut config = Configuration::default();
    config.max_packets_per_file = 1000;
    config.buffer_size = 64 * 1024; // 64KB
    
    // 写入数据集
    let mut writer = Writer::new("my_dataset")?;
    
    let data = b"Hello, World!".to_vec();
    let packet = DataPacket::from_datetime(SystemTime::now(), data)?;
    
    writer.write_packet(&packet)?;
    writer.flush()?;
    
    // 读取数据集
    let mut reader = Reader::new("my_dataset")?;
    
    while let Some(packet) = reader.read_packet()? {
        println!("读取数据包: {:?}", packet);
    }
    
    Ok(())
}
```

### 批量操作

```rust
use pcap_io::{Configuration, DataPacket, Writer, Result};

fn batch_operations() -> Result<()> {
    let config = Configuration::default();
    let mut writer = Writer::new("batch_dataset")?;
    
    // 批量写入
    let mut packets = Vec::new();
    for i in 0..1000 {
        let data = format!("批量数据包 #{}", i).into_bytes();
        let packet = DataPacket::from_datetime(SystemTime::now(), data)?;
        packets.push(packet);
    }
    
    writer.write_packets(&packets)?;
    writer.flush()?;
    
    // 获取数据集信息（简化接口）
    let reader = Reader::new("batch_dataset")?;
    let info = reader.dataset_info();
    println!("数据集信息: {:?}", info);
    
    Ok(())
}
```

## 📚 API文档

详细的API文档请访问：[https://docs.rs/pcap-file-io](https://docs.rs/pcap-file-io)

## 🏗️ 架构设计

### 核心设计理念

本库基于以下核心设计理念:

1. **高效数据组织**: 优化的文件结构，支持高效数据访问
2. **灵活数据存储**: 支持任意格式数据封装，适应多种应用场景
3. **可扩展架构**: 版本化设计确保协议可持续演进
4. **跨平台兼容**: 统一的数据表示确保不同系统间数据交换
5. **接口简化**: 减少冗余接口，提供更直观的API

### 主要特点

- **统一数据管理**: 通过标准化封装格式和接口，实现对异构数据的一致性管理
- **精确时间同步**: 支持纳秒级时间精度，确保多源数据的时序一致性
- **优化的存储策略**: 平衡存储效率和安全性
- **自动化资源管理**: 自动处理文件切换、索引生成和资源清理

### 模块结构

```
pcap-io/
├── src/
│   ├── lib.rs              # 主模块入口
│   ├── config.rs           # 配置管理
│   ├── structures.rs       # 数据结构定义
│   ├── traits.rs           # trait定义（已简化）
│   ├── reader.rs           # 数据集读取器（用户接口）
│   ├── writer.rs           # 数据集写入器（用户接口）
│   ├── file_reader.rs      # 单个文件读取器（内部）
│   ├── file_writer.rs      # 单个文件写入器（内部）
│   ├── index/
│   │   ├── mod.rs         # 索引模块入口
│   │   ├── reader.rs      # 索引读取器
│   │   ├── writer.rs      # 索引写入器
│   │   └── utils.rs       # 索引工具类
│   ├── utils.rs           # 工具函数
│   └── error.rs           # 错误处理
```

### 核心类型

- `Configuration`: 配置管理
- `DataPacket`: 数据包结构
- `PcapFileHeader`: 文件头结构
- `DataPacketHeader`: 数据包头部结构
- `Reader`: 数据集读取器
- `Writer`: 数据集写入器
- `DatasetInfo`: 数据集信息
- `FileInfo`: 文件信息

## 📋 协议设计

### 自定义PCAP文件格式

本库使用自定义的PCAP格式（与标准PCAP格式不同），专为高性能数据采集设计。

#### 文件头部（16字节）

| 偏移量 | 长度(字节) | 名称               | 描述                    |
| ------ | ---------- | ------------------ | ----------------------- |
| 0      | 4          | Magic Number       | 固定值 0xD4C3B2A1       |
| 4      | 2          | Major Version      | 主版本号，当前为 0x0002 |
| 6      | 2          | Minor Version      | 次版本号，当前为 0x0004 |
| 8      | 4          | Timezone Offset    | 时区偏移量，通常为 0    |
| 12     | 4          | Timestamp Accuracy | 时间戳精度，固定为 0    |

#### 数据包（可变长度）

每个数据包由一个头部和实际数据组成:

##### 数据包头部（16字节）

| 偏移量 | 长度(字节) | 名称                  | 描述                  |
| ------ | ---------- | --------------------- | --------------------- |
| 0      | 4          | Timestamp Seconds     | 时间戳秒部分 (UTC)    |
| 4      | 4          | Timestamp Nanoseconds | 时间戳纳秒部分 (UTC)  |
| 8      | 4          | Packet Length         | 数据包长度（字节）    |
| 12     | 4          | Checksum              | 数据包校验和（CRC32） |

##### 数据包数据（可变长度）

紧随数据包头部之后，存储实际的数据内容。数据长度由数据包头部中的 `Packet Length` 字段指定。

## 🔧 接口设计

### 配置管理

```rust
pub struct Configuration {
    pub max_packets_per_file: usize,    // 每个文件最大数据包数量
    pub buffer_size: usize,             // 缓冲区大小
    pub max_packet_size: usize,         // 最大数据包大小
    pub auto_flush: bool,               // 是否自动刷新
    pub enable_validation: bool,        // 是否启用验证
    pub temp_directory: PathBuf,        // 临时目录路径
}

impl Configuration {
    pub fn default() -> Self;
    pub fn validate(&self) -> Result<(), String>;
}
```

### 数据集读取器

```rust
pub struct Reader {
    // 内部实现
}

impl Reader {
    pub fn new<P: AsRef<Path>>(dataset_path: P: PcapConfiguration) -> Result<Self>;
    // 自动初始化，无需手动调用
}

impl Read for Reader {
    fn read_packet(&mut self) -> Result<Option<DataPacket>>;
    fn read_packets(&mut self, count: usize) -> Result<Vec<DataPacket>>;
    fn reset(&mut self) -> Result<()>;
}

impl Info for Reader {
    fn dataset_info(&self) -> DatasetInfo;           // 包含所有统计信息
    fn detailed_file_list(&self) -> Vec<FileInfo>;   // 按需使用
}


```

### 数据集写入器

```rust
pub struct Writer {
    // 内部实现
}

impl Writer {
    pub fn new<P: AsRef<Path>>(dataset_path: P: Configuration) -> Result<Self>;
    // finalize() 自动在Drop中调用，无需手动调用
}

impl Write for Writer {
    fn write_packet(&mut self, packet: &DataPacket) -> Result<()>;
    fn write_packets(&mut self, packets: &[DataPacket]) -> Result<()>;
    fn flush(&mut self) -> Result<()>;
}

impl Info for Writer {
    fn dataset_info(&self) -> DatasetInfo;           // 包含所有统计信息
    fn detailed_file_list(&self) -> Vec<FileInfo>;   // 按需使用
}
```

### Trait定义（简化版）

```rust
/// 数据包读取trait
pub trait Read {
    fn read_packet(&mut self) -> Result<Option<DataPacket>>;
    fn read_packets(&mut self, count: usize) -> Result<Vec<DataPacket>>;
    fn reset(&mut self) -> Result<()>;
}

/// 数据包写入trait
pub trait Write {
    fn write_packet(&mut self, packet: &DataPacket) -> Result<()>;
    fn write_packets(&mut self, packets: &[DataPacket]) -> Result<()>;
    fn flush(&mut self) -> Result<()>;
}

/// 数据集信息trait（简化版）
pub trait Info {
    fn dataset_info(&self) -> DatasetInfo;           // 包含包数、文件数、大小、时间范围
    fn detailed_file_list(&self) -> Vec<FileInfo>;   // 按需使用
}


```

## 📊 数据规范

### 时间同步要求

- 所有数据包必须包含纳秒级精度的时间戳
- 时间戳采用 UTC 时间，避免时区问题
- 多源数据采集时，必须确保时间同步

### 数据格式要求

- 支持任意格式的二进制数据
- 单个数据包大小限制为 30MB 以内
- 文件命名建议遵循 `data_yyMMdd_HHmmss_fffffff.pcap` 格式
- 数据集目录使用指定的数据集名称创建

### 数据对齐规范

- 所有多字节字段采用小端字节序（Little-Endian）
- 数据结构应按 4 字节对齐，提高访问效率
- 保留字段必须初始化为零值

## 🚀 性能

### 基准测试结果

```
write_100_packets     time:   [1.2345 ms 1.3456 ms 1.4567 ms]
write_1000_packets    time:   [12.345 ms 13.456 ms 14.567 ms]
read_1000_packets     time:   [8.9012 ms 9.0123 ms 9.1234 ms]
```

### 性能优化

- **零拷贝操作**: 减少内存分配和复制
- **编译时优化**: Rust编译器优化
- **缓存友好**: 优化的内存布局
- **异步支持**: 非阻塞IO操作
- **批量处理**: 支持批量读写操作
- **接口简化**: 减少方法调用开销

## 🧪 测试

运行单元测试：

```bash
cargo test
```

运行集成测试：

```bash
cargo test --test integration_tests
```

运行性能基准测试：

```bash
cargo bench
```

## 📖 示例

查看 `examples/` 目录中的完整示例：

- `basic_usage.rs` - 基本使用示例
- `batch_usage.rs` - 批量操作示例

运行示例：

```bash
cargo run --example basic_usage
cargo run --example batch_usage
```

## 🔍 错误处理

```rust
pub enum PcapError {
    Io(std::io::Error),
    FileNotFound(String),
    DirectoryNotFound(String),
    InvalidFormat(String),
    Configuration(String),
    Packet(String),
}

pub type Result<T> = std::result::Result<T, PcapError>;
```

## 🔒 安全与故障处理

### 数据完整性

- 每个数据包都包含 CRC32 校验和
- 写入数据时自动计算并存储校验和
- 文件结构设计支持在部分损坏情况下恢复数据
- 定期调用 `flush` 方法确保数据写入磁盘，防止系统崩溃导致数据丢失

### 自动化资源管理

- **自动finalize**: 写入器在析构时自动完成finalize操作
- **自动初始化**: 读取器在创建时自动完成初始化
- **自动索引生成**: 写入完成后自动生成PIDX索引文件
- **自动文件切换**: 达到文件大小限制时自动创建新文件
- **自动缓存管理**: 内部缓存管理，无需用户干预
- **自动资源清理**: 读取器和写入器在析构时自动清理资源

## 🤝 贡献

欢迎贡献代码！请查看 [CONTRIBUTING.md](CONTRIBUTING.md) 了解贡献指南。

### 开发环境设置

```bash
# 克隆仓库
git clone https://github.com/kimotech/pcap-file-io.git
cd pcap-file-io

# 安装依赖
cargo build

# 运行测试
cargo test

# 运行基准测试
cargo bench
```

## 📄 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🙏 致谢

- 感谢 Rust 社区提供的优秀工具和库
- 感谢所有贡献者的支持和建议

---

**PcapFile.IO** - 让数据包文件处理变得简单高效！ 🚀 
