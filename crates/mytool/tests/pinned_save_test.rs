use mytool::todo_state;
use todos::{Store, entity::ItemModel};

#[tokio::test]
async fn test_pinned_save() {
    println!("测试pinned状态保存功能...");

    // 获取数据库连接
    let db = todo_state::get_todo_conn().await;
    let store = Store::new(db);

    // 创建一个测试任务
    let now = chrono::Utc::now().naive_utc();
    let test_item = ItemModel {
        id: format!("test_{}", uuid::Uuid::new_v4()),
        content: "测试pinned状态保存".to_string(),
        checked: false,
        pinned: false,
        priority: None,
        project_id: None,
        section_id: None,
        parent_id: None,
        added_at: now,
        completed_at: None,
        due: None,
        labels: None,
        description: None,
        updated_at: now,
        child_order: None,
        day_order: None,
        is_deleted: false,
        collapsed: false,
        extra_data: None,
        item_type: None,
    };

    // 保存测试任务
    let saved_item = store.insert_item(test_item.clone(), true).await.unwrap();
    println!("创建测试任务: {}", saved_item.id);

    // 设置为pinned
    println!("设置为pinned...");
    store.update_item_pin(&saved_item.id, true).await.unwrap();

    // 验证pinned状态
    let item_after_pin = store.get_item(&saved_item.id).await.unwrap();
    println!("设置后pinned状态: {}", item_after_pin.pinned);
    assert_eq!(item_after_pin.pinned, true, "pinned状态应该为true");

    // 模拟应用重启（重新获取store）
    println!("模拟应用重启...");
    let db2 = todo_state::get_todo_conn().await;
    let store2 = Store::new(db2);

    // 验证重启后pinned状态
    let item_after_restart = store2.get_item(&saved_item.id).await.unwrap();
    println!("重启后pinned状态: {}", item_after_restart.pinned);
    assert_eq!(item_after_restart.pinned, true, "重启后pinned状态应该保持为true");

    // 测试取消pinned
    println!("取消pinned...");
    store2.update_item_pin(&saved_item.id, false).await.unwrap();

    // 验证取消pinned状态
    let item_after_unpin = store2.get_item(&saved_item.id).await.unwrap();
    println!("取消后pinned状态: {}", item_after_unpin.pinned);
    assert_eq!(item_after_unpin.pinned, false, "pinned状态应该为false");

    // 模拟应用重启
    println!("再次模拟应用重启...");
    let db3 = todo_state::get_todo_conn().await;
    let store3 = Store::new(db3);

    // 验证重启后取消pinned状态
    let item_after_restart2 = store3.get_item(&saved_item.id).await.unwrap();
    println!("再次重启后pinned状态: {}", item_after_restart2.pinned);
    assert_eq!(item_after_restart2.pinned, false, "重启后pinned状态应该保持为false");

    // 清理测试数据
    store3.delete_item(&saved_item.id).await.unwrap();
    println!("测试完成，清理测试数据");

    println!("✅ 所有测试通过！pinned状态在应用重启后正确保存。");
}
