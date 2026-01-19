use futures::future;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task;
// 少量任务（<100）：join_all 最简单
// 中等数量（100-1000）：tokio::spawn + Semaphore
// 大量任务（>1000）：stream::buffer_unordered
// 需要精细控制：rayon + tokio 混合
// 生产环境：考虑 tower 或 actix 框架

// 定义 Case 和 Record 结构体（补充必要字段）
#[derive(Debug, Clone)]
pub struct Case {
    pub id: String,
    // 其他字段...
}

#[derive(Debug, Clone)]
pub struct Record {
    pub case_id: String,
    pub content: String,
    // 其他字段...
}

// 定义 Job 结构体（移除 Default，改为 new 方法）
pub struct Job {
    pub jobs: Vec<&'static str>,
    pub case_list: Vec<Case>,
    pub record_list: Vec<Record>,
}

impl Job {
    // 构造函数
    pub(crate) fn new() -> Self {
        Job {
            jobs: vec![
                "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15",
                "16", "17", "18", "19", "20",
            ],
            case_list: Vec::new(),
            record_list: Vec::new(),
        }
    }

    // 模拟根据 job_id 获取用例列表（异步，&self 改为 &mut self 也可）
    async fn get_case_list_by_jobid(&self, job_id: &str) -> Vec<Case> {
        // 模拟网络/数据库请求延迟
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        vec![
            Case {
                id: format!("case_{}_{}", job_id, 1),
            },
            Case {
                id: format!("case_{}_{}", job_id, 2),
            },
        ]
    }

    // 模拟根据 case_id 获取日志记录（异步）
    async fn get_log_by_case_id(&self, case_id: &str) -> Vec<Record> {
        // 模拟网络/数据库请求延迟
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        vec![
            Record {
                case_id: case_id.to_string(),
                content: format!("log_1 for {}", case_id),
            },
            Record {
                case_id: case_id.to_string(),
                content: format!("log_2 for {}", case_id),
            },
            Record {
                case_id: case_id.to_string(),
                content: format!("log_3 for {}", case_id),
            },
        ]
    }
    // stream并发1
    pub(crate) async fn get_logs_stream(&self) {
        use futures::stream::{self, StreamExt};

        // 使用buffer_unordered限制并发数
        let case_list: Vec<_> = stream::iter(self.jobs.iter())
            .map(|job_id| async move { self.get_case_list_by_jobid(job_id).await })
            .buffer_unordered(5) // 最多同时5个并发
            .collect()
            .await;

        // 扁平化case列表
        let all_cases: Vec<_> = case_list.into_iter().flatten().collect();

        // 并发获取日志
        let record_list: Vec<_> = stream::iter(all_cases.iter())
            .map(|case| async move { self.get_log_by_case_id(&case.id).await })
            .buffer_unordered(10) // 最多同时10个并发
            .collect()
            .await;

        let all_records: Vec<_> = record_list.into_iter().flatten().collect();

        println!("获取到 {} 条记录", all_records.len());
        println!("日志详情：{:?}", all_records);
    }
    // async_std + join!（轻量级方案）
    pub async fn get_logs_async_std(&self) {
        use futures::future;
        use std::time::Instant;

        // 使用async_std的join!同时等待多个future
        let start = Instant::now();

        // 方法1: 分批并发
        let batch_size = 10;
        let mut all_cases = Vec::new();

        for chunk in self.jobs.chunks(batch_size) {
            let futures: Vec<_> = chunk
                .iter()
                .map(|job_id| self.get_case_list_by_jobid(job_id))
                .collect();

            let results = future::join_all(futures).await;
            for cases in results {
                all_cases.extend(cases);
            }
        }

        // 方法2: 使用yield_now避免阻塞
        let mut record_futures = Vec::new();
        for case in &all_cases {
            record_futures.push(self.get_log_by_case_id(&case.id));

            // 每10个任务yield一次
            if record_futures.len() % 10 == 0 {
                async_std::task::yield_now().await;
            }
        }

        let all_records = future::join_all(record_futures).await;
        let record_list: Vec<_> = all_records.into_iter().flatten().collect();

        println!("耗时: {:?}, 记录: {}", start.elapsed(), record_list.len());
    }
    // semaphore
    pub async fn get_logs_with_semaphore(&self, max_concurrent: usize) {
        let semaphore = Arc::new(Semaphore::new(max_concurrent));

        // 获取cases
        let case_futures: Vec<_> = self
            .jobs
            .iter()
            .map(|job_id| {
                let semaphore = semaphore.clone();
                async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    self.get_case_list_by_jobid(job_id).await
                }
            })
            .collect();

        let case_results = futures::future::join_all(case_futures).await;
        let all_cases: Vec<_> = case_results.into_iter().flatten().collect();

        // 获取logs
        let log_futures: Vec<_> = all_cases
            .iter()
            .map(|case| {
                let semaphore = semaphore.clone();
                let case_id = case.id.clone();
                async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    self.get_log_by_case_id(&case_id).await
                }
            })
            .collect();

        let log_results = futures::future::join_all(log_futures).await;
        let record_list: Vec<_> = log_results.into_iter().flatten().collect();

        println!(
            "并发限制: {}, 记录数: {}",
            max_concurrent,
            record_list.len()
        );
    }

    // 使用 smol（轻量级异步运行时）
    pub async fn get_logs_smol_join(&self) {
        // 使用 smol 的 block_on 在 async 环境中运行并发任务
        let all_cases = smol::block_on(async {
            let futures: Vec<_> = self
                .jobs
                .iter()
                .map(|job_id| self.get_case_list_by_jobid(job_id))
                .collect();

            let results = future::join_all(futures).await;
            results.into_iter().flatten().collect::<Vec<Case>>()
        });

        let record_list = smol::block_on(async {
            let futures: Vec<_> = all_cases
                .iter()
                .map(|case| self.get_log_by_case_id(&case.id))
                .collect();

            let results = future::join_all(futures).await;
            results.into_iter().flatten().collect::<Vec<Record>>()
        });

        println!("Got {} records", record_list.len());
    }
    pub async fn get_logs_smol_stream(&self) {
        use futures::stream::{self, StreamExt};
        use std::sync::Arc;
        let job_arc = Arc::new(self);

        // 创建 jobs 的流
        let jobs_stream = stream::iter(self.jobs.iter());

        // 第一步：并发获取 cases
        let cases_stream = jobs_stream
            .then(|job_id| {
                let job = job_arc.clone();
                let job_id = job_id.to_string();
                async move { job.get_case_list_by_jobid(&job_id).await }
            })
            .flat_map(stream::iter); // 扁平化

        // 收集所有 cases
        let all_cases: Vec<Case> = cases_stream.collect().await;

        // 第二步：并发获取 logs
        let logs_stream = stream::iter(all_cases.iter())
            .then(|case| {
                let job = job_arc.clone();
                let case_id = case.id.clone();
                async move { job.get_log_by_case_id(&case_id).await }
            })
            .flat_map(stream::iter); // 扁平化

        // 收集结果
        let record_list: Vec<Record> = logs_stream.collect().await;

        println!("Got {} records via stream", record_list.len());
    }

    // rayon + tokio（CPU密集型 + I/O密集型混合）

    pub async fn get_logs_rayon_tokio_fixed(self: Arc<Self>) {
        use rayon::prelude::*;
        use std::thread;
        use tokio::sync::mpsc;

        // 在线程中处理Rayon并行部分
        let (cases_tx, cases_rx) = std::sync::mpsc::channel();
        let value = self.clone();
        thread::spawn(move || {
            // 在新线程中创建运行时
            let rt = tokio::runtime::Runtime::new().unwrap();
            let value = value.clone();
            let all_cases: Vec<Case> = value
                .jobs
                .par_iter()
                .flat_map(|job_id| {
                    let self_clone = Arc::clone(&value);
                    rt.block_on(async move { self_clone.get_case_list_by_jobid(job_id).await })
                })
                .collect();

            // 发送结果回主线程
            let _ = cases_tx.send(all_cases);
            // 运行时在这里正确丢弃（在线程中）
        });

        // 接收cases
        let all_cases = cases_rx.recv().unwrap();

        // Stage 2: 并发获取logs
        let (tx, mut rx) = mpsc::channel(100);

        for case in all_cases {
            let tx = tx.clone();
            let self_clone = Arc::clone(&self);
            let case_id = case.id.clone();

            tokio::spawn(async move {
                let logs = self_clone.get_log_by_case_id(&case_id).await;
                let _ = tx.send(logs).await;
            });
        }

        drop(tx);

        let mut record_list = Vec::new();
        while let Some(logs) = rx.recv().await {
            record_list.extend(logs);
        }

        println!("记录数: {}", record_list.len());
    }
    pub(crate) async fn get_logs_with_existing_runtime(self: Arc<Self>) {
        // Stage 1: 并发获取cases
        let case_futures: Vec<_> = self
            .jobs
            .iter()
            .map(|job_id| {
                let self_clone = Arc::clone(&self);
                let job_id = job_id.to_string();

                async move { self_clone.get_case_list_by_jobid(&job_id).await }
            })
            .collect();

        let case_results = futures::future::join_all(case_futures).await;
        let all_cases: Vec<Case> = case_results.into_iter().flatten().collect();

        // Stage 2: 并发获取logs
        let log_futures: Vec<_> = all_cases
            .iter()
            .map(|case| {
                let self_clone = Arc::clone(&self);
                let case_id = case.id.clone();

                async move { self_clone.get_log_by_case_id(&case_id).await }
            })
            .collect();

        let log_results = futures::future::join_all(log_futures).await;
        let record_list: Vec<_> = log_results.into_iter().flatten().collect();

        println!("记录数: {}", record_list.len());
    }
    // 使用 tokio::runtime::Handle::try_current()
    pub(crate) async fn get_logs_with_handle(self: Arc<Self>) {
        // 获取当前运行时的handle
        let handle = tokio::runtime::Handle::try_current()
            .expect("Must be called from within a Tokio runtime");

        // Stage 1: 使用spawn_blocking处理可能阻塞的操作
        let case_tasks: Vec<_> = self
            .jobs
            .iter()
            .map(|job_id| {
                let handle = handle.clone();
                let self_clone = Arc::clone(&self);
                let job_id = job_id.to_string();

                task::spawn_blocking(move || {
                    // 使用已有的handle，而不是创建新的运行时
                    handle.block_on(async move { self_clone.get_case_list_by_jobid(&job_id).await })
                })
            })
            .collect();

        let mut all_cases = Vec::new();
        for task in case_tasks {
            match task.await {
                Ok(cases) => all_cases.extend(cases),
                Err(e) => eprintln!("Failed to get cases: {:?}", e),
            }
        }

        // Stage 2: 异步获取logs
        let log_futures: Vec<_> = all_cases
            .iter()
            .map(|case| {
                let self_clone = Arc::clone(&self);
                let case_id = case.id.clone();

                async move { self_clone.get_log_by_case_id(&case_id).await }
            })
            .collect();

        let log_results = futures::future::join_all(log_futures).await;
        let record_list: Vec<_> = log_results.into_iter().flatten().collect();

        println!("记录数: {}", record_list.len());
    }

    pub(crate) async fn get_logs_block_in_place(self: Arc<Self>) {
        // Stage 1: 获取cases
        let case_futures: Vec<_> = self
            .jobs
            .iter()
            .map(|job_id| {
                let self_clone = Arc::clone(&self);
                let job_id = job_id.to_string();

                // 使用 block_in_place 而不是创建新运行时
                tokio::task::block_in_place(move || {
                    // 在当前线程中阻塞执行
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();

                    rt.block_on(async move { self_clone.get_case_list_by_jobid(&job_id).await })
                })
            })
            .collect();

        let all_cases: Vec<Case> = case_futures.into_iter().flatten().collect();

        // Stage 2: 异步获取logs
        let log_futures: Vec<_> = all_cases
            .iter()
            .map(|case| {
                let self_clone = Arc::clone(&self);
                let case_id = case.id.clone();

                async move { self_clone.get_log_by_case_id(&case_id).await }
            })
            .collect();

        let log_results = futures::future::join_all(log_futures).await;
        let record_list: Vec<_> = log_results.into_iter().flatten().collect();

        println!("记录数: {}", record_list.len());
    }
}
