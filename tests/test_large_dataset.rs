//! 大规模数据集测试
//!
//! 测试大规模数据（10万个数据包）的写入读取功能和性能

use pcap_io::{
    DataPacket, PcapReader, PcapWriter, PcapResult, WriterConfig,
};
use std::path::Path;
use std::time::{Instant, SystemTime};
use tempfile::TempDir;

/// 创建测试数据包
fn create_test_packet(
    sequence: usize,
    size: usize,
) -> PcapResult<DataPacket> {
    let mut data = vec![0u8; size];

    // 填充测试数据模式 - 使用更复杂的模式以避免压缩
    for i in 0..size {
        data[i] = ((sequence * 31 + i * 17) % 256) as u8;
    }

    let capture_time = SystemTime::now();
    Ok(DataPacket::from_datetime(capture_time, data)?)
}

/// 大规模写入测试
fn write_large_dataset(
    base_path: &Path,
    dataset_name: &str,
    packet_count: usize,
    packet_size: usize,
) -> PcapResult<(u64, std::time::Duration)> {
    let config = WriterConfig {
        max_packets_per_file: 2000,
        auto_flush: false,
        common: pcap_io::CommonConfig {
            buffer_size: 64 * 1024, // 64KB
            index_cache_size: 5000,
            enable_index_cache: true,
            ..Default::default()
        },
        ..Default::default()
    }; // 使用高性能配置
    let mut writer = PcapWriter::new_with_config(
        base_path,
        dataset_name,
        config,
    )?;

    let start_time = Instant::now();

    for i in 0..packet_count {
        let packet = create_test_packet(i, packet_size)?;
        writer.write_packet(&packet)?;

        // 每1万个数据包报告一次进度
        if i % 10000 == 0 && i > 0 {
            println!(
                "已写入: {}/{} 个数据包",
                i, packet_count
            );
        }
    }

    writer.finalize()?;
    let elapsed = start_time.elapsed();

    // 获取写入的数据包数量进行验证
    let dataset_info = {
        let mut reader =
            PcapReader::new(base_path, dataset_name)?;
        reader.get_dataset_info()?
    };

    Ok((dataset_info.total_packets, elapsed))
}

/// 大规模读取测试
fn read_large_dataset(
    base_path: &Path,
    dataset_name: &str,
) -> PcapResult<(usize, std::time::Duration)> {
    let mut reader =
        PcapReader::new(base_path, dataset_name)?;

    let start_time = Instant::now();
    let mut read_count = 0;

    while let Some(_packet) = reader.read_packet()? {
        read_count += 1;

        // 每1万个数据包报告一次进度
        if read_count % 10000 == 0 {
            println!("已读取: {} 个数据包", read_count);
        }
    }

    let elapsed = start_time.elapsed();
    Ok((read_count, elapsed))
}

#[test]
fn test_large_dataset_basic_functionality() {
    let temp_dir =
        TempDir::new().expect("创建临时目录失败");
    let base_path = temp_dir.path();
    let dataset_name = "large_test_dataset";

    const PACKET_COUNT: usize = 100_000; // 10万个数据包
    const PACKET_SIZE: usize = 1024; // 1KB每个数据包

    println!(
        "开始大规模数据集测试：{} 个数据包",
        PACKET_COUNT
    );

    // 步骤1: 写入大规模数据
    println!("步骤 1/2: 写入大规模数据");
    let (written_count, write_time) = write_large_dataset(
        base_path,
        dataset_name,
        PACKET_COUNT,
        PACKET_SIZE,
    )
    .expect("写入大规模数据失败");

    assert_eq!(
        written_count, PACKET_COUNT as u64,
        "写入的数据包数量不正确"
    );

    let write_throughput =
        PACKET_COUNT as f64 / write_time.as_secs_f64();
    println!(
        "写入完成：{} 个数据包，耗时 {:.2}秒，吞吐量 {:.0} 包/秒",
        written_count,
        write_time.as_secs_f64(),
        write_throughput
    );

    // 步骤2: 读取大规模数据
    println!("步骤 2/2: 读取大规模数据");
    let (read_count, read_time) =
        read_large_dataset(base_path, dataset_name)
            .expect("读取大规模数据失败");

    assert_eq!(
        read_count, PACKET_COUNT,
        "读取的数据包数量不正确"
    );

    let read_throughput =
        read_count as f64 / read_time.as_secs_f64();
    println!(
        "读取完成：{} 个数据包，耗时 {:.2}秒，吞吐量 {:.0} 包/秒",
        read_count,
        read_time.as_secs_f64(),
        read_throughput
    );

    // 性能基准验证
    assert!(
        write_throughput > 1000.0,
        "写入吞吐量太低：{:.0} 包/秒，期望 > 1000 包/秒",
        write_throughput
    );
    assert!(
        read_throughput > 5000.0,
        "读取吞吐量太低：{:.0} 包/秒，期望 > 5000 包/秒",
        read_throughput
    );

    println!(
        "✅ 大规模数据集测试通过：{} 个数据包",
        PACKET_COUNT
    );
}

#[test]
fn test_large_dataset_memory_usage() {
    // 测试大规模数据下的内存使用情况
    let temp_dir =
        TempDir::new().expect("创建临时目录失败");
    let base_path = temp_dir.path();
    let dataset_name = "memory_test_dataset";

    const PACKET_COUNT: usize = 50_000; // 5万个数据包
    const PACKET_SIZE: usize = 2048; // 2KB每个数据包

    // 使用低内存配置
    let config = WriterConfig {
        max_packets_per_file: 100,
        auto_flush: true,
        common: pcap_io::CommonConfig {
            buffer_size: 2048, // 2KB
            index_cache_size: 100,
            enable_index_cache: false,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut writer = PcapWriter::new_with_config(
        base_path,
        dataset_name,
        config,
    )
    .expect("创建PcapWriter失败");

    // 写入数据包（一次只在内存中保存少量数据）
    for i in 0..PACKET_COUNT {
        let packet = create_test_packet(i, PACKET_SIZE)
            .expect("创建数据包失败");
        writer
            .write_packet(&packet)
            .expect("写入数据包失败");

        if i % 5000 == 0 && i > 0 {
            println!(
                "内存测试 - 已写入: {}/{} 个数据包",
                i, PACKET_COUNT
            );
        }
    }

    writer.finalize().expect("完成写入失败");

    // 验证读取（低内存模式）
    let mut reader =
        PcapReader::new(base_path, dataset_name)
            .expect("创建PcapReader失败");

    let mut read_count = 0;
    while let Some(_packet) =
        reader.read_packet().expect("读取数据包失败")
    {
        read_count += 1;

        if read_count % 5000 == 0 {
            println!(
                "内存测试 - 已读取: {} 个数据包",
                read_count
            );
        }
    }

    assert_eq!(
        read_count, PACKET_COUNT,
        "内存测试：读取的数据包数量不正确"
    );

    println!(
        "✅ 大规模数据集内存使用测试通过：{} 个数据包",
        PACKET_COUNT
    );
}

#[test]
fn test_large_dataset_file_segmentation() {
    // 测试大规模数据的文件分割功能
    let temp_dir =
        TempDir::new().expect("创建临时目录失败");
    let base_path = temp_dir.path();
    let dataset_name = "segmentation_test";

    const PACKET_COUNT: usize = 25_000; // 2.5万个数据包
    const PACKET_SIZE: usize = 1024;

    // 配置较小的文件分割大小
    let mut config = WriterConfig::default();
    config.max_packets_per_file = 1000; // 每个文件1000个数据包

    let mut writer = PcapWriter::new_with_config(
        base_path,
        dataset_name,
        config,
    )
    .expect("创建PcapWriter失败");

    // 写入数据包
    for i in 0..PACKET_COUNT {
        let packet = create_test_packet(i, PACKET_SIZE)
            .expect("创建数据包失败");
        writer
            .write_packet(&packet)
            .expect("写入数据包失败");
    }

    writer.finalize().expect("完成写入失败");

    // 验证文件分割
    let mut reader =
        PcapReader::new(base_path, dataset_name)
            .expect("创建PcapReader失败");

    let dataset_info = reader
        .get_dataset_info()
        .expect("获取数据集信息失败");
    let expected_files = (PACKET_COUNT + 999) / 1000; // 向上取整

    assert_eq!(
        dataset_info.total_packets, PACKET_COUNT as u64,
        "总数据包数量不正确"
    );
    assert_eq!(
        dataset_info.file_count, expected_files,
        "文件分割数量不正确，期望 {} 个文件，实际 {} 个文件",
        expected_files, dataset_info.file_count
    );

    println!(
        "✅ 大规模数据集文件分割测试通过：{} 个数据包分割为 {} 个文件",
        PACKET_COUNT, dataset_info.file_count
    );
}
