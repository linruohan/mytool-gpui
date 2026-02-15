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

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Record {
    case_id: String,
    content: String,
    // 其他字段...
}

// 定义 Job 结构体（移除 Default，改为 new 方法）
#[allow(dead_code)]
pub struct Job {
    pub jobs: Vec<&'static str>,
    pub case_list: Vec<Case>,
    pub record_list: Vec<Record>,
}

#[allow(dead_code)]
impl Job {
    // 统一的获取 case 和 log 的小工具，减少重复代码
    async fn collect_cases(&self, concurrency: usize) -> Vec<Case> {
        use futures::stream::{self, StreamExt};

        stream::iter(self.jobs.iter())
            .map(|job_id| async move { self.get_case_list_by_jobid(job_id).await })
            .buffer_unordered(concurrency)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .flatten()
            .collect()
    }

    async fn collect_logs(&self, cases: &[Case], concurrency: usize) -> Vec<Record> {
        use futures::stream::{self, StreamExt};

        stream::iter(cases.iter())
            .map(|case| async move { self.get_log_by_case_id(&case.id).await })
            .buffer_unordered(concurrency)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .flatten()
            .collect()
    }

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
        let all_cases = self.collect_cases(5).await;
        let all_records = self.collect_logs(&all_cases, 10).await;

        println!("获取到 {} 条记录", all_records.len());
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
        let all_cases = smol::block_on(self.collect_cases(8));
        let record_list = smol::block_on(self.collect_logs(&all_cases, 16));

        println!("Got {} records", record_list.len());
    }
    // rayon + tokio（CPU密集型 + I/O密集型混合）

    pub async fn get_logs_rayon_tokio_fixed(self: Arc<Self>) {
        use rayon::prelude::*;
        use tokio::sync::mpsc;

        // 在 Tokio 的 blocking 线程池中跑 Rayon，复用当前运行时句柄避免重复创建 Runtime
        let handle = tokio::runtime::Handle::current();
        let cases_self = Arc::clone(&self);
        let all_cases: Vec<Case> = task::spawn_blocking(move || {
            cases_self
                .jobs
                .par_iter()
                .flat_map(|job_id| {
                    let job = Arc::clone(&cases_self);
                    let job_id = job_id.to_string();
                    // 在 blocking 环境中同步等待异步调用
                    handle.block_on(async move { job.get_case_list_by_jobid(&job_id).await })
                })
                .collect()
        })
        .await
        .expect("spawn_blocking failed");

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
}
