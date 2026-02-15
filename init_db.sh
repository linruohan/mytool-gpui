#!/bin/bash
sqlite3 db.sqlite < crates/todos/setup.sql
echo "Database initialized successfully!"
