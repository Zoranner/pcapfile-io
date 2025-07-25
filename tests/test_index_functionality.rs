//! 测试有索引写入索引内容的正确性，验证PIDX索引系统

use std::fs;
use std::path::Path;

use pcap_io::{
    DataPacket, PcapReader, PcapWriter, ReaderConfig,
    PcapResult, WriterConfig,
};

const TEST_BASE_PATH: &str = "test_output";
const TEST_DATASET_NAME: &str = "test_index";

/// 设置测试环境
fn setup_test_environment() -> PcapResult<()> {
    let base_path = Path::new(TEST_BASE_PATH);
    if base_path.exists() {
        fs::remove_dir_all(base_path)
            .map_err(|e| pcap_io::PcapError::Io(e))?;
    }
    fs::create_dir_all(base_path)
        .map_err(|e| pcap_io::PcapError::Io(e))?;

    let dataset_path = base_path.join(TEST_DATASET_NAME);
    if dataset_path.exists() {
        fs::remove_dir_all(&dataset_path)
            .map_err(|e| pcap_io::PcapError::Io(e))?;
    }
    fs::create_dir_all(&dataset_path)
        .map_err(|e| pcap_io::PcapError::Io(e))?;

    Ok(())
}

/// 创建测试数据包
fn create_test_packet(
    sequence: u32,
    size: usize,
) -> PcapResult<DataPacket> {
    let mut data = vec![0u8; size];

    // 填充测试数据
    for i in 0..size {
        data[i] = (i + sequence as usize) as u8;
    }

    let capture_time = std::time::SystemTime::now();
    Ok(DataPacket::from_datetime(capture_time, data)?)
}

#[test]
fn test_index_generation_and_loading() {
    setup_test_environment().expect("设置测试环境失败");

    const PACKET_COUNT: usize = 5000;
    const PACKET_SIZE: usize = 1024;

    let dataset_path =
        Path::new(TEST_BASE_PATH).join(TEST_DATASET_NAME);

    // 步骤1: 创建数据集并启用索引
    let mut config = WriterConfig::default();
    config.common.enable_index_cache = true; // 启用索引缓存

    let mut writer = PcapWriter::new_with_config(
        &dataset_path,
        TEST_DATASET_NAME,
        config,
    )
    .expect("创建PcapWriter失败");

    let mut written_timestamps = Vec::new();
    for i in 0..PACKET_COUNT {
        let packet =
            create_test_packet(i as u32, PACKET_SIZE)
                .expect("创建测试数据包失败");
        written_timestamps.push(packet.get_timestamp_ns());
        writer
            .write_packet(&packet)
            .expect("写入数据包失败");
    }

    // 完成写入，这将自动生成索引
    writer.finalize().expect("完成PcapWriter失败");

    // 步骤2: 通过Reader验证自动生成的索引
    let mut reader = PcapReader::new_with_config(
        &dataset_path,
        TEST_DATASET_NAME,
        ReaderConfig::default(),
    )
    .expect("创建PcapReader失败");
    reader.initialize().expect("初始化Reader失败");

    // 通过 reader.index() 访问索引
    let index =
        reader.index().get_index().expect("获取索引失败");

    // 验证索引内容
    assert_eq!(index.total_packets, PACKET_COUNT as u64);
    assert!(index.data_files.files.len() > 0);

    // 验证时间戳索引
    assert_eq!(index.timestamp_index.len(), PACKET_COUNT);

    println!(
        "索引验证通过: {} 个数据包",
        index.total_packets
    );
}

#[test]
fn test_manual_index_generation() {
    setup_test_environment().expect("设置测试环境失败");

    const PACKET_COUNT: usize = 3000;
    const PACKET_SIZE: usize = 512;

    let dataset_path =
        Path::new(TEST_BASE_PATH).join(TEST_DATASET_NAME);

    // 步骤1: 创建数据集但不启用自动索引
    let mut config = WriterConfig::default();
    config.common.enable_index_cache = false; // 禁用自动索引

    let mut writer = PcapWriter::new_with_config(
        &dataset_path,
        TEST_DATASET_NAME,
        config,
    )
    .expect("创建PcapWriter失败");

    for i in 0..PACKET_COUNT {
        let packet =
            create_test_packet(i as u32, PACKET_SIZE)
                .expect("创建测试数据包失败");
        writer
            .write_packet(&packet)
            .expect("写入数据包失败");
    }

    writer.finalize().expect("完成PcapWriter失败");

    // 步骤2: 手动生成索引
    let mut reader = PcapReader::new_with_config(
        &dataset_path,
        TEST_DATASET_NAME,
        ReaderConfig::default(),
    )
    .expect("创建PcapReader失败");

    // 通过 reader.index_mut() 手动生成索引
    let index_path = reader
        .regenerate_index()
        .expect("手动生成索引失败");

    // 步骤3: 验证手动生成的索引
    let index =
        reader.index().get_index().expect("获取索引失败");

    assert_eq!(index.total_packets, PACKET_COUNT as u64);
    assert!(index.data_files.files.len() > 0);

    // 验证索引与数据集信息的一致性
    let dataset_info = reader
        .get_dataset_info()
        .expect("获取数据集信息失败");
    assert_eq!(
        dataset_info.total_packets,
        PACKET_COUNT as u64
    );

    println!("手动索引生成成功: {:?}", index_path);
}

#[test]
fn test_index_content_verification() {
    setup_test_environment().expect("设置测试环境失败");

    const PACKET_COUNT: usize = 2000;
    const PACKET_SIZE: usize = 256;

    let dataset_path =
        Path::new(TEST_BASE_PATH).join(TEST_DATASET_NAME);

    // 创建具有已知时间戳的数据包
    let mut writer = PcapWriter::new_with_config(
        &dataset_path,
        TEST_DATASET_NAME,
        WriterConfig::default(),
    )
    .expect("创建PcapWriter失败");

    let mut expected_timestamps = Vec::new();
    for i in 0..PACKET_COUNT {
        let packet =
            create_test_packet(i as u32, PACKET_SIZE)
                .expect("创建测试数据包失败");
        expected_timestamps.push(packet.get_timestamp_ns());
        writer
            .write_packet(&packet)
            .expect("写入数据包失败");
    }

    writer.finalize().expect("完成PcapWriter失败");

    // 读取并验证索引内容
    let mut reader = PcapReader::new_with_config(
        &dataset_path,
        TEST_DATASET_NAME,
        ReaderConfig::default(),
    )
    .expect("创建PcapReader失败");
    reader.initialize().expect("初始化Reader失败");

    let index =
        reader.index().get_index().expect("获取索引失败");

    // 验证索引中的时间戳范围
    assert!(index.start_timestamp > 0);
    assert!(index.end_timestamp >= index.start_timestamp);

    // 验证时间戳索引包含预期的时间戳
    for expected_ts in &expected_timestamps {
        assert!(
            index.timestamp_index.contains_key(expected_ts),
            "索引中缺少时间戳: {}",
            expected_ts
        );
    }

    println!("索引内容验证通过");
}

#[test]
fn test_index_query_functionality() {
    setup_test_environment().expect("设置测试环境失败");

    const PACKET_COUNT: usize = 1500;
    const PACKET_SIZE: usize = 512;

    let dataset_path =
        Path::new(TEST_BASE_PATH).join(TEST_DATASET_NAME);

    // 写入数据包
    let mut writer = PcapWriter::new_with_config(
        &dataset_path,
        TEST_DATASET_NAME,
        WriterConfig::default(),
    )
    .expect("创建PcapWriter失败");

    for i in 0..PACKET_COUNT {
        let packet =
            create_test_packet(i as u32, PACKET_SIZE)
                .expect("创建测试数据包失败");
        writer
            .write_packet(&packet)
            .expect("写入数据包失败");
    }

    writer.finalize().expect("完成PcapWriter失败");

    // 加载索引
    let mut reader = PcapReader::new_with_config(
        &dataset_path,
        TEST_DATASET_NAME,
        ReaderConfig::default(),
    )
    .expect("创建PcapReader失败");
    reader.initialize().expect("初始化Reader失败");

    let index =
        reader.index().get_index().expect("获取索引失败");

    // 测试索引查询功能（如果提供的话）
    assert!(index.timestamp_index.len() > 0);

    // 验证数据包计数
    assert_eq!(index.total_packets, PACKET_COUNT as u64);

    // 验证索引是否需要重建
    let needs_rebuild = reader
        .index()
        .needs_rebuild()
        .expect("检查重建状态失败");
    assert!(!needs_rebuild, "新生成的索引不应该需要重建");

    println!("索引查询功能测试通过");
}

#[test]
fn test_index_rebuild_detection() {
    setup_test_environment().expect("设置测试环境失败");

    const PACKET_COUNT: usize = 1000;
    const PACKET_SIZE: usize = 256;

    let dataset_path =
        Path::new(TEST_BASE_PATH).join(TEST_DATASET_NAME);

    // 创建数据集并生成索引
    let mut writer = PcapWriter::new_with_config(
        &dataset_path,
        TEST_DATASET_NAME,
        WriterConfig::default(),
    )
    .expect("创建PcapWriter失败");

    for i in 0..PACKET_COUNT {
        let packet =
            create_test_packet(i as u32, PACKET_SIZE)
                .expect("创建测试数据包失败");
        writer
            .write_packet(&packet)
            .expect("写入数据包失败");
    }

    writer.finalize().expect("完成PcapWriter失败");

    // 创建Reader并初始化
    let mut reader = PcapReader::new_with_config(
        &dataset_path,
        TEST_DATASET_NAME,
        ReaderConfig::default(),
    )
    .expect("创建PcapReader失败");
    reader.initialize().expect("初始化Reader失败");

    // 索引应该是有效的
    let needs_rebuild = reader
        .index()
        .needs_rebuild()
        .expect("检查重建状态失败");
    assert!(!needs_rebuild, "新生成的索引应该是有效的");

    // 模拟数据文件变化（添加新文件）
    let new_file_path =
        dataset_path.join("additional.pcap");
    fs::write(&new_file_path, b"dummy content")
        .expect("创建新文件失败");

    // 现在索引应该需要重建
    let needs_rebuild = reader
        .index()
        .needs_rebuild()
        .expect("检查重建状态失败");
    assert!(needs_rebuild, "添加新文件后索引应该需要重建");

    println!("索引重建检测测试通过");
}
