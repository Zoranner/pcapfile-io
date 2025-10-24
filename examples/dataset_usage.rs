//! 数据集使用示例
//!
//! 演示如何使用PcapFile.IO库进行数据集读写操作，包括：
//! - 通过Writer创建数据集并生成索引
//! - 通过Reader读取数据集并访问索引信息
//! - 索引管理和验证
//! - 使用自定义时间戳生成数据包
//! - 生成随机大小和内容的数据包

use chrono::{DateTime, TimeZone, Utc};
use pcapfile_io::{
    DataPacket, PcapReader, PcapResult, PcapWriter,
    ReaderConfig, WriterConfig,
};
use rand::Rng;
use std::path::Path;

// ========================================
// 测试配置参数（可根据需要修改）
// ========================================

/// 数据集名称
const DATASET_NAME: &str = "test_dataset_04";

/// 开始时间：年
const START_YEAR: i32 = 2025;
/// 开始时间：月
const START_MONTH: u32 = 10;
/// 开始时间：日
const START_DAY: u32 = 1;
/// 开始时间：时
const START_HOUR: u32 = 8;
/// 开始时间：分
const START_MINUTE: u32 = 30;
/// 开始时间：秒
const START_SECOND: u32 = 10;

/// 每秒生成的数据包数量
const PACKETS_PER_SECOND: usize = 8;
/// 持续时间（秒）
const DURATION_SECONDS: usize = 2000;
/// 数据包大小范围：最小值（字节）
const MIN_PACKET_SIZE: usize = 64;
/// 数据包大小范围：最大值（字节）
const MAX_PACKET_SIZE: usize = 1500;
/// 每个文件最大数据包数
const MAX_PACKETS_PER_FILE: usize = 8000;

fn main() -> PcapResult<()> {
    // 设置数据集路径
    let dataset_path = Path::new("examples/output");

    // 确保输出目录存在
    std::fs::create_dir_all(dataset_path)?;

    // 如果特定数据集已存在，则删除
    let specific_dataset_path = dataset_path.join(DATASET_NAME);
    if specific_dataset_path.exists() {
        std::fs::remove_dir_all(&specific_dataset_path)?;
    }

    println!("=== PcapFile.IO 数据集使用示例 ===\n");

    // 第一步：创建数据集并写入数据包
    create_dataset(dataset_path)?;

    // 第二步：读取数据集并访问索引信息
    read_dataset(dataset_path)?;

    // 第三步：演示索引管理功能
    demonstrate_index_management(dataset_path)?;

    println!("\n=== 示例完成 ===");
    Ok(())
}

/// 创建测试数据包
fn create_test_packet(
    capture_time: DateTime<Utc>,
) -> PcapResult<DataPacket> {
    let mut rng = rand::thread_rng();

    // 随机生成数据包大小
    let size =
        rng.gen_range(MIN_PACKET_SIZE..=MAX_PACKET_SIZE);
    let mut data = vec![0u8; size];

    // 填充随机数据
    rng.fill(&mut data[..]);

    Ok(DataPacket::from_datetime(capture_time, data)?)
}

/// 创建数据集并写入数据包
fn create_dataset(dataset_path: &Path) -> PcapResult<()> {
    println!("1. 创建数据集并写入数据包...");

    // 配置写入器
    let config = WriterConfig {
        max_packets_per_file: MAX_PACKETS_PER_FILE,
        ..Default::default()
    };

    let mut writer = PcapWriter::new_with_config(
        dataset_path,
        DATASET_NAME,
        config,
    )?;

    // 使用配置的开始时间
    let start_time = Utc
        .with_ymd_and_hms(
            START_YEAR,
            START_MONTH,
            START_DAY,
            START_HOUR,
            START_MINUTE,
            START_SECOND,
        )
        .unwrap();

    println!(
        "   开始时间: {}",
        start_time.format("%Y-%m-%d %H:%M:%S")
    );

    // 计算总数据包数
    const TOTAL_PACKETS: usize =
        PACKETS_PER_SECOND * DURATION_SECONDS;

    // 每个数据包之间的时间间隔（纳秒）
    const INTERVAL_NANOSECONDS: i64 =
        1_000_000_000 / PACKETS_PER_SECOND as i64;

    println!("   配置: 每秒{PACKETS_PER_SECOND}个数据包，持续{DURATION_SECONDS}秒");
    println!("   总数据包数: {TOTAL_PACKETS}");
    println!("   数据包大小: {MIN_PACKET_SIZE}-{MAX_PACKET_SIZE} 字节（随机）");
    println!(
        "   数据包间隔: {:.2} 毫秒\n",
        INTERVAL_NANOSECONDS as f64 / 1_000_000.0
    );

    // 写入数据包
    for i in 0..TOTAL_PACKETS {
        // 计算当前数据包的时间戳
        let packet_time = start_time
            + chrono::Duration::nanoseconds(
                i as i64 * INTERVAL_NANOSECONDS,
            );

        let packet = create_test_packet(packet_time)?;
        writer.write_packet(&packet)?;

        if (i + 1) % 300 == 0 {
            let count = i + 1;
            let elapsed_seconds =
                (i + 1) / PACKETS_PER_SECOND;
            println!("   已写入 {count} 个数据包 (已模拟 {elapsed_seconds} 秒)");
        }
    }

    // 完成写入（自动生成索引）
    writer.finalize()?;

    // 通过writer访问索引信息
    println!("\n   数据集信息：");
    let dataset_info = writer.get_dataset_info();
    println!(
        "     - 文件数量: {}",
        dataset_info.file_count
    );
    println!(
        "     - 数据包总数: {}",
        dataset_info.total_packets
    );
    println!(
        "     - 数据集大小: {} 字节",
        dataset_info.total_size
    );

    println!("   ✅ 数据集创建完成\n");
    Ok(())
}

/// 读取数据集并访问索引信息
fn read_dataset(dataset_path: &Path) -> PcapResult<()> {
    println!("2. 读取数据集并访问索引信息...");

    let mut reader = PcapReader::new_with_config(
        dataset_path,
        DATASET_NAME,
        ReaderConfig::default(),
    )?;

    // 获取数据集信息
    let dataset_info = reader.get_dataset_info()?;
    println!("   数据集基本信息：");
    println!(
        "     - 文件数量: {}",
        dataset_info.file_count
    );
    println!(
        "     - 数据包总数: {}",
        dataset_info.total_packets
    );
    println!(
        "     - 数据集大小: {} 字节",
        dataset_info.total_size
    );

    // 读取所有数据包并验证时间戳
    let mut packet_count = 0;
    let mut first_timestamp: Option<DateTime<Utc>> = None;
    let mut last_timestamp: Option<DateTime<Utc>> = None;

    while let Some(packet) = reader.read_packet()? {
        packet_count += 1;

        if first_timestamp.is_none() {
            first_timestamp = Some(packet.capture_time());
        }
        last_timestamp = Some(packet.capture_time());

        if packet_count % 300 == 0 {
            println!("   已读取 {packet_count} 个数据包");
        }
    }

    println!("\n   总共读取: {packet_count} 个数据包");

    if let (Some(first), Some(last)) =
        (first_timestamp, last_timestamp)
    {
        println!("   时间戳范围：");
        println!(
            "     - 第一个数据包: {}",
            first.format("%Y-%m-%d %H:%M:%S%.3f")
        );
        println!(
            "     - 最后一个数据包: {}",
            last.format("%Y-%m-%d %H:%M:%S%.3f")
        );
        let duration =
            (last.timestamp_nanos_opt().unwrap_or(0)
                - first.timestamp_nanos_opt().unwrap_or(0))
                as f64
                / 1_000_000_000.0;
        println!("     - 时间跨度: {:.3} 秒", duration);
    }

    println!("   ✅ 数据集读取完成\n");
    Ok(())
}

/// 演示索引管理功能
fn demonstrate_index_management(
    dataset_path: &Path,
) -> PcapResult<()> {
    println!("3. 演示索引管理功能...");

    let mut reader =
        PcapReader::new(dataset_path, DATASET_NAME)?;

    // 获取详细文件信息
    let file_list = reader.get_file_info_list()?;
    println!("   文件详情：");
    for (i, file_info) in file_list.iter().enumerate() {
        println!(
            "     文件 {}: {} ({} 数据包, {} 字节)",
            i + 1,
            file_info.file_name,
            file_info.packet_count,
            file_info.file_size
        );
    }

    // 显示数据集统计信息
    let dataset_info = reader.get_dataset_info()?;
    if let (Some(start), Some(end)) = (
        dataset_info.start_timestamp,
        dataset_info.end_timestamp,
    ) {
        println!("   时间范围：");
        println!("     - 开始时间戳: {start} ns");
        println!("     - 结束时间戳: {end} ns");
        println!(
            "     - 时间跨度: {:.2} 秒",
            (end - start) as f64 / 1_000_000_000.0
        );
    }

    println!("   ✅ 索引管理演示完成\n");
    Ok(())
}
