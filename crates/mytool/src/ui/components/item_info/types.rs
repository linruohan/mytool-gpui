#[derive(Clone)]
pub enum ItemInfoEvent {
    Updated(),       // 更新任务
    Added(),         // 新增任务
    Finished(),      // 状态改为完成
    UnFinished(),    // 状态改为未完成
    Deleted(),       // 删除任务
    Cancelled(),     // 取消编辑（不保存）
    SaveSucceeded(), // 🚀 7.0: 异步保存成功
    SaveFailed(),    // 🚀 7.0: 异步保存失败
}
