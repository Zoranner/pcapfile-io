//! 文件信息缓存管理模块
//!
//! 提供高效的文件信息缓存策略，减少重复的文件系统访问，提升性能。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use crate::data::models::FileInfo;

/// 缓存统计信息
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// 缓存条目总数
    pub total_entries: usize,
    /// 缓存命中次数
    pub hit_count: u64,
    /// 缓存未命中次数
    pub miss_count: u64,
    /// 缓存命中率
    pub hit_rate: f64,
}

impl CacheStats {
    /// 创建新的缓存统计信息
    pub fn new() -> Self {
        Self {
            total_entries: 0,
            hit_count: 0,
            miss_count: 0,
            hit_rate: 0.0,
        }
    }

    /// 更新命中率
    pub fn update_hit_rate(&mut self) {
        let total_requests = self.hit_count + self.miss_count;
        self.hit_rate = if total_requests > 0 {
            self.hit_count as f64 / total_requests as f64
        } else {
            0.0
        };
    }
}

/// 文件信息缓存项
#[derive(Debug, Clone)]
pub struct FileInfoCacheItem {
    pub file_info: FileInfo,
    pub cache_time: SystemTime,
}

impl FileInfoCacheItem {
    pub fn new(file_info: FileInfo) -> Self {
        Self {
            file_info,
            cache_time: SystemTime::now(),
        }
    }

    pub fn is_valid(&self, current_file_size: u64, current_write_time: SystemTime) -> bool {
        self.file_info.file_size == current_file_size
            && self.file_info.modified_time
                == current_write_time
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
                    .to_string()
    }

    pub fn is_expired(&self, expiration_duration: Duration) -> bool {
        SystemTime::now()
            .duration_since(self.cache_time)
            .unwrap_or(Duration::ZERO)
            >= expiration_duration
    }
}

/// 缓存统计信息
#[derive(Debug, Clone)]
pub struct CacheStatistics {
    pub total_entries: usize,
    pub max_entries: usize,
    pub expired_entries: usize,
    pub last_cleanup_time: SystemTime,
}

impl CacheStatistics {
    pub fn usage_percentage(&self) -> f64 {
        if self.max_entries == 0 {
            0.0
        } else {
            (self.total_entries as f64 / self.max_entries as f64) * 100.0
        }
    }
}

/// 文件信息缓存
pub struct FileInfoCache {
    cache: Arc<Mutex<HashMap<String, FileInfoCacheItem>>>,
    max_entries: usize,
    cache_expiration: Duration,
    cleanup_interval: Duration,
    last_cleanup: Arc<Mutex<SystemTime>>,
    hit_count: Arc<Mutex<u64>>,
    miss_count: Arc<Mutex<u64>>,
}

impl FileInfoCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            max_entries,
            cache_expiration: Duration::from_secs(30 * 60), // 30分钟
            cleanup_interval: Duration::from_secs(10 * 60), // 10分钟
            last_cleanup: Arc::new(Mutex::new(SystemTime::now())),
            hit_count: Arc::new(Mutex::new(0)),
            miss_count: Arc::new(Mutex::new(0)),
        }
    }

    /// 从缓存中获取文件信息
    pub fn get<P: AsRef<std::path::Path>>(&self, file_path: P) -> Option<FileInfo> {
        let path_str = file_path.as_ref().to_string_lossy().to_string();
        let mut cache = self.cache.lock().ok()?;

        // 执行定期清理
        let _ = self.perform_periodic_cleanup(&mut cache);

        if let Some(item) = cache.get(&path_str) {
            // 检查文件是否已修改
            if let Ok(metadata) = std::fs::metadata(&file_path) {
                if let Ok(modified_time) = metadata.modified() {
                    if item.is_valid(metadata.len(), modified_time) {
                        // 缓存命中
                        if let Ok(mut hit_count) = self.hit_count.lock() {
                            *hit_count += 1;
                        }
                        return Some(item.file_info.clone());
                    }
                }
            }
        }

        // 缓存未命中
        if let Ok(mut miss_count) = self.miss_count.lock() {
            *miss_count += 1;
        }

        None
    }

    /// 向缓存中插入文件信息
    pub fn insert<P: AsRef<std::path::Path>>(&self, file_path: P, file_info: FileInfo) {
        let path_str = file_path.as_ref().to_string_lossy().to_string();

        if let Ok(mut cache) = self.cache.lock() {
            let item = FileInfoCacheItem::new(file_info);
            cache.insert(path_str, item);

            // 检查缓存大小限制
            if cache.len() > self.max_entries {
                let _ = self.cleanup_expired_entries(&mut cache);

                // 如果清理后仍然超过限制，移除最旧的条目
                if cache.len() > self.max_entries {
                    let oldest_key = cache
                        .iter()
                        .min_by_key(|(_, item)| item.cache_time)
                        .map(|(key, _)| key.clone());

                    if let Some(key) = oldest_key {
                        cache.remove(&key);
                    }
                }
            }
        }
    }

    /// 获取缓存统计信息
    pub fn get_cache_stats(&self) -> CacheStats {
        let total_entries = self.cache.lock().map(|cache| cache.len()).unwrap_or(0);

        let hit_count = self.hit_count.lock().map(|guard| *guard).unwrap_or(0);
        let miss_count = self.miss_count.lock().map(|guard| *guard).unwrap_or(0);

        let mut stats = CacheStats {
            total_entries,
            hit_count,
            miss_count,
            hit_rate: 0.0,
        };

        stats.update_hit_rate();
        stats
    }

    fn perform_periodic_cleanup(
        &self,
        cache: &mut HashMap<String, FileInfoCacheItem>,
    ) -> Result<(), String> {
        let mut last_cleanup = self.last_cleanup.lock().map_err(|_| "清理时间锁定失败")?;
        let now = SystemTime::now();

        if now.duration_since(*last_cleanup).unwrap_or(Duration::ZERO) >= self.cleanup_interval {
            self.cleanup_expired_entries(cache)?;
            *last_cleanup = now;
        }

        Ok(())
    }

    fn cleanup_expired_entries(
        &self,
        cache: &mut HashMap<String, FileInfoCacheItem>,
    ) -> Result<(), String> {
        let expired_keys: Vec<String> = cache
            .iter()
            .filter(|(_, item)| item.is_expired(self.cache_expiration))
            .map(|(key, _)| key.clone())
            .collect();

        for key in expired_keys {
            cache.remove(&key);
        }

        Ok(())
    }

    pub fn invalidate_file(&self, file_path: &str) -> Result<(), String> {
        let mut cache = self.cache.lock().map_err(|_| "缓存锁定失败")?;
        cache.remove(file_path);
        Ok(())
    }

    pub fn clear(&self) -> Result<(), String> {
        let mut cache = self.cache.lock().map_err(|_| "缓存锁定失败")?;
        cache.clear();
        Ok(())
    }

    pub fn get_statistics(&self) -> Result<CacheStatistics, String> {
        let cache = self.cache.lock().map_err(|_| "缓存锁定失败")?;

        let expired_entries = cache
            .values()
            .filter(|item| item.is_expired(self.cache_expiration))
            .count();

        let last_cleanup = *self.last_cleanup.lock().map_err(|_| "清理时间锁定失败")?;

        Ok(CacheStatistics {
            total_entries: cache.len(),
            max_entries: self.max_entries,
            expired_entries,
            last_cleanup_time: last_cleanup,
        })
    }
}

impl Default for FileInfoCache {
    fn default() -> Self {
        Self::new(1000)
    }
}
