use chrono::Utc;
use criterion::{
    black_box, criterion_group, criterion_main,
    BenchmarkId, Criterion,
};
use pcapfile_io::{DataPacket, PcapWriter};
use tempfile::TempDir;

/// 创建测试数据包
fn create_test_packet(size: usize) -> DataPacket {
    let data = vec![0u8; size];
    DataPacket::from_datetime(Utc::now(), data)
        .expect("创建数据包失败")
}

/// 基准测试：单个数据包写入
fn bench_write_packet(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_packet");

    for size in [64, 256, 1024, 4096].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, &size| {
                b.iter(|| {
                    let temp_dir = TempDir::new()
                        .expect("创建临时目录失败");
                    let mut writer = PcapWriter::new(
                        temp_dir.path(),
                        "bench_dataset",
                    )
                    .expect("创建Writer失败");

                    let packet = create_test_packet(size);
                    writer
                        .write_packet(&packet)
                        .expect("写入失败");

                    black_box(());
                });
            },
        );
    }

    group.finish();
}

/// 基准测试：批量写入数据包
fn bench_write_packets_batch(c: &mut Criterion) {
    let mut group =
        c.benchmark_group("write_packets_batch");

    for count in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            count,
            |b, &count| {
                b.iter(|| {
                    let temp_dir = TempDir::new()
                        .expect("创建临时目录失败");
                    let mut writer = PcapWriter::new(
                        temp_dir.path(),
                        "bench_dataset",
                    )
                    .expect("创建Writer失败");

                    // 创建批量数据包
                    let packets: Vec<DataPacket> = (0
                        ..count)
                        .map(|i| {
                            let data =
                                format!("Packet #{}", i)
                                    .into_bytes();
                            DataPacket::from_datetime(
                                Utc::now(),
                                data,
                            )
                            .expect("创建失败")
                        })
                        .collect();

                    writer
                        .write_packets(&packets)
                        .expect("批量写入失败");

                    black_box(());
                });
            },
        );
    }

    group.finish();
}

/// 基准测试：大量小数据包写入
fn bench_write_many_small_packets(c: &mut Criterion) {
    c.bench_function("write_1000_small_packets", |b| {
        b.iter(|| {
            let temp_dir =
                TempDir::new().expect("创建临时目录失败");
            let mut writer = PcapWriter::new(
                temp_dir.path(),
                "bench_dataset",
            )
            .expect("创建Writer失败");

            // 写入1000个小数据包
            for i in 0..1000 {
                let data = format!("Small packet #{}", i)
                    .into_bytes();
                let packet = DataPacket::from_datetime(
                    Utc::now(),
                    data,
                )
                .expect("创建失败");
                writer
                    .write_packet(&packet)
                    .expect("写入失败");
            }

            writer.flush().expect("刷新失败");

            black_box(());
        });
    });
}

/// 基准测试：写入后立即刷新 vs 批量刷新
fn bench_write_with_flush(c: &mut Criterion) {
    let mut group =
        c.benchmark_group("write_flush_strategy");

    // 每次写入后立即刷新
    group.bench_function("flush_each", |b| {
        b.iter(|| {
            let temp_dir =
                TempDir::new().expect("创建临时目录失败");
            let mut writer = PcapWriter::new(
                temp_dir.path(),
                "bench_dataset",
            )
            .expect("创建Writer失败");

            for i in 0..100 {
                let data =
                    format!("Packet #{}", i).into_bytes();
                let packet = DataPacket::from_datetime(
                    Utc::now(),
                    data,
                )
                .expect("创建失败");
                writer
                    .write_packet(&packet)
                    .expect("写入失败");
                writer.flush().expect("刷新失败");
            }

            black_box(());
        });
    });

    // 批量写入后一次性刷新
    group.bench_function("flush_batch", |b| {
        b.iter(|| {
            let temp_dir =
                TempDir::new().expect("创建临时目录失败");
            let mut writer = PcapWriter::new(
                temp_dir.path(),
                "bench_dataset",
            )
            .expect("创建Writer失败");

            for i in 0..100 {
                let data =
                    format!("Packet #{}", i).into_bytes();
                let packet = DataPacket::from_datetime(
                    Utc::now(),
                    data,
                )
                .expect("创建失败");
                writer
                    .write_packet(&packet)
                    .expect("写入失败");
            }
            writer.flush().expect("刷新失败");

            black_box(());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_write_packet,
    bench_write_packets_batch,
    bench_write_many_small_packets,
    bench_write_with_flush
);
criterion_main!(benches);
