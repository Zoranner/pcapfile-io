use chrono::{Duration, Utc};
use criterion::{
    black_box, criterion_group, criterion_main, Criterion,
};
use pcapfile_io::{DataPacket, PcapReader, PcapWriter};
use std::path::PathBuf;
use tempfile::TempDir;

/// 设置测试环境（带时间戳的数据）
fn setup_test_data_with_timestamps(
) -> (TempDir, PathBuf, String, Vec<u64>) {
    let temp_dir =
        TempDir::new().expect("创建临时目录失败");
    let base_path = temp_dir.path().to_path_buf();
    let dataset_name = "bench_dataset";

    let mut writer =
        PcapWriter::new(&base_path, dataset_name)
            .expect("创建Writer失败");

    let base_time = Utc::now();
    let mut timestamps = Vec::new();

    // 创建1000个数据包，每个间隔1ms
    for i in 0..1000 {
        let timestamp =
            base_time + Duration::milliseconds(i);
        let data = format!("Timestamped packet #{}", i)
            .into_bytes();
        let packet =
            DataPacket::from_datetime(timestamp, data)
                .expect("创建数据包失败");

        timestamps.push(packet.get_timestamp_ns());

        writer
            .write_packet(&packet)
            .expect("写入数据包失败");
    }

    writer.finalize().expect("完成写入失败");

    (
        temp_dir,
        base_path,
        dataset_name.to_string(),
        timestamps,
    )
}

/// 基准测试：索引生成
fn bench_index_generation(c: &mut Criterion) {
    c.bench_function("index_generation", |b| {
        b.iter(|| {
            let (_temp_dir, base_path, dataset_name, _) =
                setup_test_data_with_timestamps();

            let mut reader =
                PcapReader::new(&base_path, &dataset_name)
                    .expect("创建Reader失败");

            // 触发索引生成
            reader
                .index_mut()
                .regenerate_index()
                .expect("生成索引失败");

            black_box(());
        });
    });
}

/// 基准测试：精确时间戳查找
fn bench_find_by_exact_timestamp(c: &mut Criterion) {
    let (_temp_dir, base_path, dataset_name, timestamps) =
        setup_test_data_with_timestamps();

    c.bench_function("find_by_exact_timestamp", |b| {
        b.iter(|| {
            let mut reader =
                PcapReader::new(&base_path, &dataset_name)
                    .expect("创建Reader失败");
            reader.initialize().expect("初始化失败");

            // 查找中间的时间戳
            let target_timestamp = timestamps[500];
            let _packet = reader
                .read_packet_by_timestamp(target_timestamp)
                .expect("查找失败");

            black_box(_packet);
        });
    });
}

/// 基准测试：时间范围查询
fn bench_read_time_range(c: &mut Criterion) {
    let (_temp_dir, base_path, dataset_name, timestamps) =
        setup_test_data_with_timestamps();

    let mut group = c.benchmark_group("read_time_range");

    // 不同范围大小的查询
    for range_size in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            criterion::BenchmarkId::from_parameter(
                range_size,
            ),
            range_size,
            |b, &range_size| {
                b.iter(|| {
                    let mut reader = PcapReader::new(
                        &base_path,
                        &dataset_name,
                    )
                    .expect("创建Reader失败");
                    reader
                        .initialize()
                        .expect("初始化失败");

                    let start_idx = 100;
                    let end_idx = start_idx + range_size;

                    let start_time = timestamps[start_idx];
                    let end_time = timestamps
                        [end_idx.min(timestamps.len() - 1)];

                    let packets = reader
                        .read_packets_by_time_range(
                            start_time, end_time,
                        )
                        .expect("时间范围查询失败");

                    black_box(packets);
                });
            },
        );
    }

    group.finish();
}

/// 基准测试：跨文件随机访问
fn bench_random_access(c: &mut Criterion) {
    let (_temp_dir, base_path, dataset_name, timestamps) =
        setup_test_data_with_timestamps();

    c.bench_function("random_access", |b| {
        b.iter(|| {
            let mut reader =
                PcapReader::new(&base_path, &dataset_name)
                    .expect("创建Reader失败");
            reader.initialize().expect("初始化失败");

            // 随机访问多个时间戳
            let access_indices = [100, 500, 200, 800, 300];

            for &idx in &access_indices {
                let timestamp = timestamps[idx];
                let _packet = reader
                    .read_packet_by_timestamp(timestamp)
                    .expect("随机访问失败");
                black_box(_packet);
            }
        });
    });
}

/// 基准测试：索引验证
fn bench_index_validation(c: &mut Criterion) {
    let (_temp_dir, base_path, dataset_name, _) =
        setup_test_data_with_timestamps();

    c.bench_function("index_validation", |b| {
        b.iter(|| {
            let mut reader =
                PcapReader::new(&base_path, &dataset_name)
                    .expect("创建Reader失败");
            reader.initialize().expect("初始化失败");

            // 验证索引
            let _is_valid = reader
                .index()
                .verify_index_validity()
                .expect("验证失败");

            black_box(_is_valid);
        });
    });
}

criterion_group!(
    benches,
    bench_index_generation,
    bench_find_by_exact_timestamp,
    bench_read_time_range,
    bench_random_access,
    bench_index_validation
);
criterion_main!(benches);
