use chrono::Utc;
use criterion::{
    black_box, criterion_group, criterion_main,
    BenchmarkId, Criterion,
};
use pcapfile_io::{DataPacket, PcapReader, PcapWriter};
use std::path::PathBuf;
use tempfile::TempDir;

/// 设置测试环境
fn setup_test_data() -> (TempDir, PathBuf, String) {
    let temp_dir =
        TempDir::new().expect("创建临时目录失败");
    let base_path = temp_dir.path().to_path_buf();
    let dataset_name = "bench_dataset";

    // 创建测试数据
    let mut writer =
        PcapWriter::new(&base_path, dataset_name)
            .expect("创建Writer失败");

    // 写入1000个测试数据包
    for i in 0..1000 {
        let data =
            format!("Benchmark packet #{}", i).into_bytes();
        let packet =
            DataPacket::from_datetime(Utc::now(), data)
                .expect("创建数据包失败");
        writer
            .write_packet(&packet)
            .expect("写入数据包失败");
    }

    writer.finalize().expect("完成写入失败");

    (temp_dir, base_path, dataset_name.to_string())
}

/// 基准测试：单个数据包读取
fn bench_read_packet(c: &mut Criterion) {
    let (_temp_dir, base_path, dataset_name) =
        setup_test_data();

    c.bench_function("read_packet", |b| {
        b.iter(|| {
            let mut reader =
                PcapReader::new(&base_path, &dataset_name)
                    .expect("创建Reader失败");
            reader.initialize().expect("初始化失败");

            // 读取一个数据包
            let _packet = reader
                .read_packet()
                .expect("读取数据包失败");

            black_box(_packet);
        });
    });
}

/// 基准测试：批量读取数据包
fn bench_read_packets_batch(c: &mut Criterion) {
    let (_temp_dir, base_path, dataset_name) =
        setup_test_data();

    let mut group = c.benchmark_group("read_packets_batch");

    for count in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            count,
            |b, &count| {
                b.iter(|| {
                    let mut reader = PcapReader::new(
                        &base_path,
                        &dataset_name,
                    )
                    .expect("创建Reader失败");
                    reader
                        .initialize()
                        .expect("初始化失败");

                    // 批量读取
                    let packets = reader
                        .read_packets(count)
                        .expect("批量读取失败");

                    black_box(packets);
                });
            },
        );
    }

    group.finish();
}

/// 基准测试：仅读取数据（不关心校验结果）
fn bench_read_packet_data_only(c: &mut Criterion) {
    let (_temp_dir, base_path, dataset_name) =
        setup_test_data();

    c.bench_function("read_packet_data_only", |b| {
        b.iter(|| {
            let mut reader =
                PcapReader::new(&base_path, &dataset_name)
                    .expect("创建Reader失败");
            reader.initialize().expect("初始化失败");

            // 读取一个数据包（仅数据）
            let _packet = reader
                .read_packet_data_only()
                .expect("读取数据包失败");

            black_box(_packet);
        });
    });
}

/// 基准测试：顺序读取所有数据包
fn bench_read_all_sequential(c: &mut Criterion) {
    let (_temp_dir, base_path, dataset_name) =
        setup_test_data();

    c.bench_function("read_all_sequential", |b| {
        b.iter(|| {
            let mut reader =
                PcapReader::new(&base_path, &dataset_name)
                    .expect("创建Reader失败");
            reader.initialize().expect("初始化失败");

            // 顺序读取所有数据包
            let mut count = 0;
            while let Some(_packet) =
                reader.read_packet().expect("读取失败")
            {
                count += 1;
            }

            black_box(count);
        });
    });
}

criterion_group!(
    benches,
    bench_read_packet,
    bench_read_packets_batch,
    bench_read_packet_data_only,
    bench_read_all_sequential
);
criterion_main!(benches);
