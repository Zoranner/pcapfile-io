//! 数据集使用示例
//!
//! 演示如何使用PCAP-IO库进行数据集读写操作，包括：
//! - 通过Writer创建数据集并生成索引
//! - 通过Reader读取数据集并访问索引信息
//! - 索引管理和验证

use pcap_io::{
    DataPacket, PcapReader, PcapWriter, ReaderConfig,
    PcapResult, WriterConfig,
};
use std::{path::Path, time::SystemTime};

fn main() -> PcapResult<()> {
    // 设置数据集路径
    let dataset_path =
        Path::new("example_output").join("sample_dataset");

    // 确保目录存在
    if dataset_path.exists() {
        std::fs::remove_dir_all(&dataset_path)?;
    }
    std::fs::create_dir_all(&dataset_path)?;

    println!("=== PCAP-IO 数据集使用示例 ===\n");

    // 第一步：创建数据集并写入数据包
    create_dataset(&dataset_path)?;

    // 第二步：读取数据集并访问索引信息
    read_dataset(&dataset_path)?;

    // 第三步：演示索引管理功能
    demonstrate_index_management(&dataset_path)?;

    println!("\n=== 示例完成 ===");
    Ok(())
}

/// 创建测试数据包
fn create_test_packet(
    sequence: usize,
    size: usize,
) -> PcapResult<DataPacket> {
    let mut data = vec![0u8; size];

    // 填充测试数据模式
    for i in 0..size {
        data[i] = ((sequence + i) % 256) as u8;
    }

    let capture_time = SystemTime::now();
    Ok(DataPacket::from_datetime(capture_time, data)?)
}

/// 创建数据集并写入数据包
fn create_dataset(dataset_path: &Path) -> PcapResult<()> {
    println!("1. 创建数据集并写入数据包...");

    // 配置写入器
    let mut config = WriterConfig::default();
    config.common.enable_index_cache = true; // 启用自动索引生成
    config.max_packets_per_file = 1000; // 每1000个数据包一个文件

    let mut writer = PcapWriter::new_with_config(
        dataset_path,
        "test_dataset",
        config,
    )?;

    // 写入2500个测试数据包
    for i in 0..2500 {
        let packet = create_test_packet(i, 256)?;
        writer.write_packet(&packet)?;

        if i % 500 == 0 {
            println!("   已写入 {} 个数据包", i + 1);
        }
    }

    // 完成写入（自动生成索引）
    writer.finalize()?;

    // 通过writer访问索引信息
    println!("   数据集信息：");
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
        "test_dataset",
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

    // 读取所有数据包
    let mut packet_count = 0;
    while let Some(_packet) = reader.read_packet()? {
        packet_count += 1;

        if packet_count % 500 == 0 {
            println!("   已读取 {} 个数据包", packet_count);
        }
    }

    println!("   总共读取: {} 个数据包", packet_count);
    println!("   ✅ 数据集读取完成\n");
    Ok(())
}

/// 演示索引管理功能
fn demonstrate_index_management(
    dataset_path: &Path,
) -> PcapResult<()> {
    println!("3. 演示索引管理功能...");

    let mut reader =
        PcapReader::new(dataset_path, "test_dataset")?;

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
        println!("     - 开始时间戳: {} ns", start);
        println!("     - 结束时间戳: {} ns", end);
        println!(
            "     - 时间跨度: {:.2} 秒",
            (end - start) as f64 / 1_000_000_000.0
        );
    }

    println!("   ✅ 索引管理演示完成\n");
    Ok(())
}
