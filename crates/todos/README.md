# diesel demo

## 环境准备

### 安装diesel-cli工具

cargo install diesel_cli --no-default-features --features sqlite

### 设置环境变量 使用sqlite

<!-- echo DATABASE_URL=postgres://username:password@localhost/diesel_demo > .env -->

echo DATABASE_URL=db.sqlite > .env

## diesel 初始化

## diesel初始化,生成db.sqlite文件

diesel setup

## 初始化 sql 空模板

diesel migration generate create_todos_table

创建和删除表的sql

```sql
# up.sql
CREATE TABLE todos (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    content TEXT NOT NULL
);
# down.sql
DROP TABLE todos
```

## 生成schema文件

diesel print-schema >src/schema.rs
![pic1](screenshot/image.png)

## 测试

### cargo run 启动服务： 127.0.0.1:5002

![alt text](screenshot/run.png)

### 执行测试

```bash
# Create a todo
curl -X POST -H "Content-Type: application/json" -d '{"title":"Buy groceries","content":"banana,milk"}' http://localhost:5002/todos
# List all todos
curl http://localhost:5002/todos
# Get a specific todo
curl http://localhost:5002/todos/1
# Update a todo
curl -X POST -H "Content-Type: application/json" -d '{"title":"Buy Groceries", "content": "banana"}' http://localhost:5002/todos/1
# Delete a todo
curl -X DELETE http://localhost:5002/todos/1
```

1. create_todo:
   ![create_todo](screenshot/create.png)
2. get_todos
   ![create_todo](screenshot/todos.png)
3. get_todo
   ![create_todo](screenshot/todo.png)
4. update_todo
   ![create_todo](screenshot/update.png)
5. delete_todo
   ![create_todo](screenshot/delete.png)

## 表格字段更新后重新迁移

1. 修改sql后执行，检查sql语句是否正确
   diesel migration redo
2. 执行生成表格动作
   diesel migration run

## wasm 使用sqlite

Compile wasm and start the web server:

```bash
rustup target add wasm32-unknown-unknown
# Add wasm32-unknown-unknown toolchain

cargo install wasm-pack
# Install the wasm-pack toolchain

wasm-pack build --target web
# Build wasm

python3 server.py
# Start server
```

Next, try it on the web page: [on the web page](http://localhost:8000)
