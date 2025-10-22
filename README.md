# PcapFile.IO - 高性能数据包文件读写库

[![Crates.io](https://img.shields.io/crates/v/pcapfile-io)](https://crates.io/crates/pcapfile-io)
[![Documentation](https://docs.rs/pcapfile-io/badge.svg)](https://docs.rs/pcapfile-io)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org)

一个用 Rust 编写的高性能数据包文件处理库，提供完整的数据包文件读写功能。本库使用自定义的 PCAP 格式，专为高性能数据采集和回放设计。

## ✨ 核心特性

- 🚀 **高性能**: 零拷贝操作和编译时优化
- 🔒 **内存安全**: Rust 的内存安全保证
- 🧵 **线程安全**: 内置线程安全支持
- 📦 **易于使用**: 简洁直观的 API 设计
- ⚙️ **灵活配置**: 丰富的配置选项
- ✅ **数据完整性**: 内置 CRC32 校验和验证
- 🛡️ **错误恢复**: 支持跳过损坏数据包继续处理
- 🌍 **跨平台**: 支持 Windows、Linux、macOS

## 📦 安装

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
pcapfile-io = "0.1.4"
chrono = "0.4"  # 用于时间戳处理
```

## 🚀 快速开始

### 基本读写操作

```rust
use pcapfile_io::{PcapReader, PcapWriter, DataPacket};
use chrono::Utc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 写入数据包
    {
        let mut writer = PcapWriter::new("./data", "my_dataset")?;

        // 创建数据包
        let data = b"Hello, World!".to_vec();
        let packet = DataPacket::from_datetime(Utc::now(), data)?;

        writer.write_packet(&packet)?;
        writer.flush()?;
    } // writer 自动完成 finalize

    // 读取数据包（默认带校验结果）
    {
        let mut reader = PcapReader::new("./data", "my_dataset")?;

        while let Some(validated_packet) = reader.read_packet()? {
            if validated_packet.is_valid() {
                println!("读取到有效数据包: {} 字节", validated_packet.packet_length());
                println!("时间戳: {}", validated_packet.capture_time());
            } else {
                println!("读取到损坏数据包: {} 字节 (继续处理)", validated_packet.packet_length());
            }
        }
    }

    Ok(())
}
```

### 仅读取数据（不关心校验结果）

```rust
use pcapfile_io::{PcapReader, DataPacket};

fn read_data_only() -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = PcapReader::new("./data", "my_dataset")?;

    // 如果不关心校验结果，可以直接获取数据包
    while let Some(packet) = reader.read_packet_data_only()? {
        println!("读取到数据包: {} 字节", packet.packet_length());
        // 注意：这种方式仍然进行校验，只是不返回校验结果
    }

    Ok(())
}
```

### 批量操作

```rust
use pcapfile_io::{PcapWriter, DataPacket};
use chrono::Utc;

fn batch_operations() -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = PcapWriter::new("./data", "batch_dataset")?;

    // 批量写入
    let mut packets = Vec::new();
    for i in 0..1000 {
        let data = format!("数据包 #{}", i).into_bytes();
        let packet = DataPacket::from_datetime(Utc::now(), data)?;
        packets.push(packet);
    }

    writer.write_packets(&packets)?;
    writer.flush()?;

    Ok(())
}
```

## 📖 API 文档

### 核心类型

#### `DataPacket` - 数据包

```rust
pub struct DataPacket {
    pub header: DataPacketHeader,
    pub data: Vec<u8>,
}

impl DataPacket {
    // 创建方法
    pub fn from_datetime(capture_time: DateTime<Utc>, data: Vec<u8>) -> Result<Self, String>;
    pub fn from_timestamp(timestamp_seconds: u32, timestamp_nanoseconds: u32, data: Vec<u8>) -> Result<Self, String>;

    // 访问方法
    pub fn capture_time(&self) -> DateTime<Utc>;
    pub fn packet_length(&self) -> usize;
    pub fn checksum(&self) -> u32;
    pub fn is_valid(&self) -> bool;  // 内部校验
}
```

#### `ValidatedPacket` - 带校验结果的数据包

```rust
pub struct ValidatedPacket {
    pub packet: DataPacket,
    pub is_valid: bool,  // 校验是否通过
}

impl ValidatedPacket {
    pub fn is_valid(&self) -> bool;
    pub fn is_invalid(&self) -> bool;

    // 委托给内部数据包的方法
    pub fn packet_length(&self) -> usize;
    pub fn capture_time(&self) -> DateTime<Utc>;
    pub fn get_timestamp_ns(&self) -> u64;
    pub fn checksum(&self) -> u32;
}
```

### 读取器 API

#### `PcapReader` - 数据集读取器

```rust
pub struct PcapReader { /* ... */ }

impl PcapReader {
    // 构造方法
    pub fn new<P: AsRef<Path>>(base_path: P, dataset_name: &str) -> PcapResult<Self>;
    pub fn new_with_config<P: AsRef<Path>>(base_path: P, dataset_name: &str, config: ReaderConfig) -> PcapResult<Self>;

    // 初始化方法
    pub fn initialize(&mut self) -> PcapResult<()>;

    // 默认读取方法（带校验结果）
    pub fn read_packet(&mut self) -> PcapResult<Option<ValidatedPacket>>;
    pub fn read_packets(&mut self, count: usize) -> PcapResult<Vec<ValidatedPacket>>;

    // 仅数据读取方法（不返回校验信息）
    pub fn read_packet_data_only(&mut self) -> PcapResult<Option<DataPacket>>;
    pub fn read_packets_data_only(&mut self, count: usize) -> PcapResult<Vec<DataPacket>>;

    // 控制方法
    pub fn reset(&mut self) -> PcapResult<()>;

    // 定位和导航方法
    pub fn seek_to_timestamp(&mut self, timestamp_ns: u64) -> PcapResult<u64>;
    pub fn seek_to_packet(&mut self, packet_index: usize) -> PcapResult<()>;
    pub fn skip_packets(&mut self, count: usize) -> PcapResult<usize>;

    // 状态查询方法
    pub fn is_eof(&self) -> bool;
    pub fn total_packets(&self) -> Option<usize>;
    pub fn current_packet_index(&self) -> u64;
    pub fn progress(&self) -> Option<f64>;

    // 信息查询
    pub fn get_dataset_info(&mut self) -> PcapResult<DatasetInfo>;
    pub fn get_file_info_list(&mut self) -> PcapResult<Vec<FileInfo>>;
    pub fn dataset_path(&self) -> &Path;
    pub fn dataset_name(&self) -> &str;

    // 索引和缓存管理
    pub fn index(&self) -> &IndexManager;
    pub fn index_mut(&mut self) -> &mut IndexManager;
    pub fn get_cache_stats(&self) -> CacheStats;
    pub fn clear_cache(&mut self) -> PcapResult<()>;
}
```

### 写入器 API

#### `PcapWriter` - 数据集写入器

```rust
pub struct PcapWriter { /* ... */ }

impl PcapWriter {
    // 构造方法
    pub fn new<P: AsRef<Path>>(base_path: P, dataset_name: &str) -> PcapResult<Self>;
    pub fn new_with_config<P: AsRef<Path>>(base_path: P, dataset_name: &str, config: WriterConfig) -> PcapResult<Self>;

    // 初始化方法
    pub fn initialize(&mut self) -> PcapResult<()>;
    pub fn finalize(&mut self) -> PcapResult<()>;  // 手动完成，也可在 Drop 时自动调用

    // 写入方法
    pub fn write_packet(&mut self, packet: &DataPacket) -> PcapResult<()>;
    pub fn write_packets(&mut self, packets: &[DataPacket]) -> PcapResult<()>;

    // 控制方法
    pub fn flush(&mut self) -> PcapResult<()>;

    // 信息查询
    pub fn get_dataset_info(&self) -> DatasetInfo;
    pub fn get_file_info_list(&self) -> Vec<FileInfo>;
    pub fn dataset_path(&self) -> &Path;
    pub fn dataset_name(&self) -> &str;

    // 索引和缓存管理
    pub fn index(&self) -> &IndexManager;
    pub fn index_mut(&mut self) -> &mut IndexManager;
    pub fn get_cache_stats(&self) -> CacheStats;
    pub fn clear_cache(&mut self) -> PcapResult<()>;
}
```

### 配置选项

#### `ReaderConfig` - 读取器配置

```rust
pub struct ReaderConfig {
    pub buffer_size: usize,        // 缓冲区大小（字节）
    pub index_cache_size: usize,   // 索引缓存大小（条目数）
}

impl ReaderConfig {
    pub fn default() -> Self;
    pub fn validate(&self) -> Result<(), String>;  // 验证配置有效性
    pub fn reset(&mut self);                       // 重置为默认值
}
```

#### `WriterConfig` - 写入器配置

```rust
pub struct WriterConfig {
    pub buffer_size: usize,             // 缓冲区大小（字节）
    pub index_cache_size: usize,        // 索引缓存大小（条目数）
    pub max_packets_per_file: usize,    // 每个文件最大数据包数
    pub file_name_format: String,       // 文件命名格式
    pub auto_flush: bool,               // 自动刷新
}

impl WriterConfig {
    pub fn default() -> Self;
    pub fn validate(&self) -> Result<(), String>;  // 验证配置有效性
    pub fn reset(&mut self);                       // 重置为默认值
}
```

## 🔧 高级功能

### 数据校验与错误处理

本库提供了灵活的数据校验和错误处理机制：

1. **自动校验**: 每个数据包都包含 CRC32 校验和，读取时自动验证
2. **损坏数据处理**: 遇到校验失败的数据包时，不会中断读取过程
3. **校验结果反馈**: 通过 `ValidatedPacket` 类型获取校验结果

```rust
// 处理可能损坏的数据集（默认方法）
let mut reader = PcapReader::new("./data", "dataset")?;
let mut valid_count = 0;
let mut invalid_count = 0;

while let Some(validated_packet) = reader.read_packet()? {
    if validated_packet.is_valid() {
        valid_count += 1;
        // 处理有效数据包
        process_packet(&validated_packet.packet);
    } else {
        invalid_count += 1;
        // 记录损坏的数据包，但继续处理
        log::warn!("发现损坏数据包，时间戳: {}", validated_packet.capture_time());

        // 可选择是否仍然使用损坏的数据
        if should_use_corrupted_data() {
            process_packet(&validated_packet.packet);
        }
    }
}

println!("处理完成: {} 有效, {} 损坏", valid_count, invalid_count);
```

### 性能优化配置

```rust
// 使用默认配置（推荐）
let mut writer = PcapWriter::new("./data", "my_dataset")?;

// 或者自定义配置
use pcapfile_io::WriterConfig;
let mut config = WriterConfig::default();
config.buffer_size = 64 * 1024;        // 64KB 缓冲区
config.max_packets_per_file = 2000;    // 每文件 2000 个数据包
config.auto_flush = false;             // 关闭自动刷新

// 验证配置
if let Err(e) = config.validate() {
    eprintln!("配置验证失败: {}", e);
}

let mut writer = PcapWriter::new_with_config("./data", "my_dataset", config)?;
```

### 数据集信息查询

```rust
let mut reader = PcapReader::new("./data", "my_dataset")?;
let info = reader.get_dataset_info()?;

println!("数据集: {}", info.name);
println!("文件数: {}", info.file_count);
println!("总数据包数: {}", info.total_packets);
println!("总大小: {} 字节", info.total_size);
println!("时间范围: {:?}", info.time_range());
println!("平均速率: {:.2} 包/秒", info.average_packet_rate());
```

### 定位和导航

支持高效的随机访问和定位，适用于回放系统、数据采样等场景：

```rust
let mut reader = PcapReader::new("./data", "my_dataset")?;
reader.initialize()?;

// 查询数据集状态
println!("总数据包数: {:?}", reader.total_packets());
println!("当前位置: {}", reader.current_packet_index());
println!("读取进度: {:.1}%", reader.progress().unwrap_or(0.0) * 100.0);

// 按时间戳跳转（纳秒精度）
let target_ts = 1234567890_000_000_000;
let actual_ts = reader.seek_to_timestamp(target_ts)?;
println!("已跳转到时间戳: {}ns", actual_ts);

// 按数据包索引跳转
reader.seek_to_packet(1000)?;  // 跳转到第1000个数据包

// 快速跳过多个数据包
let skipped = reader.skip_packets(100)?;
println!("跳过了 {} 个数据包", skipped);

// 判断是否到达末尾
if reader.is_eof() {
    println!("已读取完毕");
}

// 重置到开头
reader.reset()?;
```

**性能特点**：
- 时间戳定位：O(1) 复杂度，基于 HashMap 索引
- 按索引定位：O(文件数) 复杂度，通常文件数很小
- 相比从头读取，性能提升 **10-100 倍**

## 📋 文件格式规范

### 自定义 PCAP 格式

本库使用自定义的 PCAP 格式，针对高性能场景优化：

#### 文件头部（16 字节）

| 偏移量 | 长度 | 字段名             | 描述                |
| ------ | ---- | ------------------ | ------------------- |
| 0      | 4    | Magic Number       | 固定值 `0xD4C3B2A1` |
| 4      | 2    | Major Version      | 主版本号 `0x0002`   |
| 6      | 2    | Minor Version      | 次版本号 `0x0004`   |
| 8      | 4    | Timezone Offset    | 时区偏移量（秒）    |
| 12     | 4    | Timestamp Accuracy | 时间戳精度（纳秒）  |

#### 数据包格式

每个数据包包含：

- **数据包头部**（16 字节）
- **数据内容**（可变长度）

##### 数据包头部（16 字节）

| 偏移量 | 长度 | 字段名                | 描述                  |
| ------ | ---- | --------------------- | --------------------- |
| 0      | 4    | Timestamp Seconds     | 时间戳秒部分（UTC）   |
| 4      | 4    | Timestamp Nanoseconds | 时间戳纳秒部分（UTC） |
| 8      | 4    | Packet Length         | 数据包长度（字节）    |
| 12     | 4    | Checksum              | 数据包校验和（CRC32） |

### 文件组织结构

```
dataset_name/
├── data_20231201_120000_123456789.pcap  # 数据文件
├── data_20231201_120100_987654321.pcap  # 数据文件
├── ...
└── dataset_name.pidx                    # 索引文件（自动生成）
```

## 🧪 测试

运行所有测试：

```bash
cargo test
```

运行特定测试：

```bash
cargo test test_data_consistency
cargo test test_large_dataset
```

运行基准测试：

```bash
cargo bench
```

## 📊 性能基准

基于 Criterion.rs 框架的性能测试结果：

### 读取性能

| 操作类型 | 平均延迟 | 单包成本 |
|---------|---------|---------|
| 单包完整读取（含校验） | 32.9ms | - |
| 单包数据读取（仅数据） | 22.1ms | - |
| 批量读取 10 包 | 19.9ms | 2.0ms/包 |
| 批量读取 100 包 | 31.0ms | 0.31ms/包 |
| 顺序读取全部 | 23.6ms | - |

### 写入性能

| 操作类型 | 平均延迟 | 单包成本 |
|---------|---------|---------|
| 单包写入 64B | 6.0ms | - |
| 单包写入 4KB | 5.0ms | - |
| 批量写入 10 包 | 5.6ms | 0.56ms/包 |
| 批量写入 100 包 | 6.5ms | 0.065ms/包 |

### 索引与查询性能

| 操作类型 | 平均延迟 |
|---------|---------|
| 索引生成 | 18.6ms |
| 索引验证 | 20.1ms |
| 随机访问 | 19.6ms |
| 精确时间戳查找 | 34.8ms |
| 时间范围查询 10 包 | 27.4ms |
| 时间范围查询 100 包 | 21.2ms |

### 运行基准测试

```bash
# 运行所有基准测试
cargo bench

# 运行特定基准测试
cargo bench --bench read_performance
cargo bench --bench write_performance
cargo bench --bench index_performance

# 查看测试报告
# 报告位于: target/criterion/report/index.html
```

### 性能优化建议

**批量操作优先**
```rust
// ✅ 推荐：批量写入
writer.write_packets(&packets)?;

// ❌ 避免：逐个写入
for packet in &packets {
    writer.write_packet(packet)?;
}
```

**控制刷新频率**
```rust
let mut config = WriterConfig::default();
config.auto_flush = false;  // 关闭自动刷新

let mut writer = PcapWriter::new_with_config("./data", "dataset", config)?;

// 批量写入后手动刷新
for chunk in packets.chunks(1000) {
    writer.write_packets(chunk)?;
}
writer.flush()?;
```

**合理配置缓冲区**
```rust
let mut config = ReaderConfig::default();
config.buffer_size = 64 * 1024;  // 64KB 缓冲区，适合大数据包
config.index_cache_size = 5000;   // 增大索引缓存

let reader = PcapReader::new_with_config("./data", "dataset", config)?;
```

## 🔍 错误处理

```rust
use pcapfile_io::{PcapError, PcapResult, PcapErrorCode};

// 主要错误类型
pub enum PcapError {
    FileNotFound(String),
    DirectoryNotFound(String),
    InvalidFormat(String),
    CorruptedHeader(String),
    CorruptedData { message: String, position: u64 },
    ChecksumMismatch { expected: String, actual: String, position: u64 },
    InvalidPacketSize { message: String, position: u64 },
    PacketSizeExceedsRemainingBytes { expected: u32, remaining: u64, position: u64 },
    TimestampParseError { message: String, position: u64 },
    InvalidArgument(String),
    InvalidState(String),
    Io(std::io::Error),
    Serialization(String),
    Unknown(String),
}

// 错误代码枚举
pub enum PcapErrorCode {
    Unknown = 0,
    FileNotFound = 1001,
    DirectoryNotFound = 1002,
    InvalidFormat = 2001,
    CorruptedHeader = 2002,
    CorruptedData = 2003,
    ChecksumMismatch = 2004,
    InvalidPacketSize = 3001,
    InvalidArgument = 3002,
    InvalidState = 3003,
}

// 结果类型
pub type PcapResult<T> = Result<T, PcapError>;

// 错误处理示例
match result {
    Ok(data) => println!("操作成功: {:?}", data),
    Err(PcapError::FileNotFound(path)) => {
        eprintln!("文件未找到: {}", path);
    }
    Err(PcapError::CorruptedData { message, position }) => {
        eprintln!("数据损坏: {}，位置: {}", message, position);
    }
    Err(e) => {
        eprintln!("错误代码: {}, 详细信息: {}", e.error_code(), e);
    }
}
```

## 📚 示例项目

查看 `examples/` 目录中的完整示例：

```bash
# 基本使用
cargo run --example dataset_usage
```

### 常见问题

**Q: 如何处理损坏的数据包？**

A: 库会自动跳过损坏的数据包并继续处理，通过 `ValidatedPacket` 可以知道哪些包损坏了：

```rust
while let Some(validated_packet) = reader.read_packet()? {
    if validated_packet.is_valid() {
        // 处理有效数据包
    } else {
        log::warn!("发现损坏数据包，跳过");
    }
}
```

**Q: 如何优化大数据集的读取性能？**

A: 使用批量读取和合适的配置：

```rust
let mut config = ReaderConfig::default();
config.buffer_size = 64 * 1024;    // 增大缓冲区
config.index_cache_size = 10000;   // 增大索引缓存

let mut reader = PcapReader::new_with_config("./data", "dataset", config)?;

// 批量读取
let packets = reader.read_packets(1000)?;
```

**Q: 索引文件何时生成？**

A: 索引文件在以下情况自动生成：
- 第一次读取数据集时
- 索引文件不存在或损坏时
- 可以手动调用 `rebuild_index()` 强制重新生成

**Q: 如何按时间范围查询数据包？**

A: 使用时间戳索引功能：

```rust
let mut reader = PcapReader::new("./data", "dataset")?;
reader.initialize()?;

// 定义时间范围（纳秒）
let start_time = start_datetime.timestamp() as u64 * 1_000_000_000;
let end_time = end_datetime.timestamp() as u64 * 1_000_000_000;

// 读取时间范围内的所有数据包
let packets = reader.read_packets_by_time_range(start_time, end_time)?;
```

## 🤝 贡献指南

我们欢迎各种形式的贡献！

### 开发环境设置

```bash
# 克隆项目
git clone https://github.com/Zoranner/pcapfile-io.git
cd pcapfile-io

# 安装依赖
cargo build

# 运行测试
cargo test

# 检查代码格式
cargo fmt --check
cargo clippy
```

## 🔗 相关链接

- [API 文档](https://docs.rs/pcapfile-io)
- [Crates.io](https://crates.io/crates/pcapfile-io)
- [问题反馈](https://github.com/Zoranner/pcapfile-io/issues)

---

**PcapFile.IO** - 让数据包文件处理变得简单高效！ 🚀
