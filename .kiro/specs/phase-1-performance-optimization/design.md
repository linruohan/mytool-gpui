# Design Document: Phase 1 Performance Optimization

## Overview

This design document details the technical approach for Phase 1 of the MyTool GPUI performance optimization. The optimization focuses on four key areas:

1. **Incremental Index Updates**: Replacing O(n) full index rebuilds with O(1) targeted updates
2. **Observer Pattern Optimization**: Adding version tracking and dirty flags to eliminate unnecessary re-renders
3. **Database Connection Management**: Using Arc<DatabaseConnection> to prevent connection cloning overhead
4. **Performance Monitoring**: Implementing comprehensive timing instrumentation for critical operations

The design maintains backward compatibility with existing code while establishing a foundation for future optimizations. All changes are localized to the TodoStore and Board view components, minimizing risk and enabling incremental rollout.

## Architecture

### Current Architecture

```
┌─────────────────────────────────────┐
│         TodoStore (Global)          │
│  ┌──────────────────────────────┐   │
│  │  all_items: Vec<Arc<Item>>   │   │
│  │  projects: Vec<Arc<Project>> │   │
│  │  labels: Vec<Arc<Label>>     │   │
│  └──────────────────────────────┘   │
│  ┌──────────────────────────────┐   │
│  │  Indexes (HashMap/HashSet)   │   │
│  │  - project_index             │   │
│  │  - section_index             │   │
│  │  - checked_set               │   │
│  │  - pinned_set                │   │
│  └──────────────────────────────┘   │
│                                     │
│  rebuild_indexes() called on        │
│  every change - O(n) complexity     │
└─────────────────────────────────────┘
          ↓ observe_global (always triggers)
┌─────────────────────────────────────┐
│          Board Views                │
│  - InboxBoard                       │
│  - TodayBoard                       │
│  - ScheduledBoard                   │
│  - ProjectBoard                     │
│                                     │
│  All views recompute on any change  │
└─────────────────────────────────────┘
```

### Optimized Architecture

```
┌─────────────────────────────────────┐
│         TodoStore (Global)          │
│  ┌──────────────────────────────┐   │
│  │  all_items: Vec<Arc<Item>>   │   │
│  │  projects: Vec<Arc<Project>> │   │
│  │  version: usize              │   │  ← NEW: Version tracking
│  └──────────────────────────────┘   │
│  ┌──────────────────────────────┐   │
│  │  Indexes (HashMap/HashSet)   │   │
│  │  Incrementally updated       │   │  ← OPTIMIZED: O(1) updates
│  └──────────────────────────────┘   │
│  ┌──────────────────────────────┐   │
│  │  Event System                │   │  ← NEW: Fine-grained events
│  │  TodoStoreEvent enum         │   │
│  └──────────────────────────────┘   │
└─────────────────────────────────────┘
          ↓ observe_global (with version check)
┌─────────────────────────────────────┐
│          Board Views                │
│  ┌──────────────────────────────┐   │
│  │  cached_version: usize       │   │  ← NEW: Version cache
│  │  Skip recompute if unchanged │   │
│  └──────────────────────────────┘   │
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│          DBState                    │
│  conn: Arc<DatabaseConnection>     │  ← OPTIMIZED: Arc wrapper
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│      PerformanceMonitor             │  ← NEW: Timing instrumentation
│  track_operation(name, fn)         │
└─────────────────────────────────────┘
```

## Components and Interfaces

### 1. TodoStore Enhancements

#### Version Tracking

```rust
pub struct TodoStore {
    // Existing fields
    pub all_items: Vec<Arc<ItemModel>>,
    pub projects: Vec<Arc<ProjectModel>>,
    pub labels: Vec<Arc<LabelModel>>,
    pub sections: Vec<Arc<SectionModel>>,
    pub active_project: Option<Arc<ProjectModel>>,
    
    // Existing indexes
    project_index: HashMap<String, Vec<Arc<ItemModel>>>,
    section_index: HashMap<String, Vec<Arc<ItemModel>>>,
    checked_set: HashSet<String>,
    pinned_set: HashSet<String>,
    
    // NEW: Version tracking
    version: usize,
}

impl TodoStore {
    /// Get the current version number
    pub fn version(&self) -> usize {
        self.version
    }
    
    /// Increment version (called internally on all mutations)
    fn increment_version(&mut self) {
        self.version = self.version.wrapping_add(1);
    }
}
```

#### Incremental Index Update Methods

```rust
impl TodoStore {
    /// Add item to indexes (O(1) operation)
    fn add_to_indexes(&mut self, item: &Arc<ItemModel>) {
        // Project index
        if let Some(project_id) = &item.project_id {
            if !project_id.is_empty() {
                self.project_index
                    .entry(project_id.clone())
                    .or_default()
                    .push(item.clone());
            }
        }
        
        // Section index
        if let Some(section_id) = &item.section_id {
            if !section_id.is_empty() {
                self.section_index
                    .entry(section_id.clone())
                    .or_default()
                    .push(item.clone());
            }
        }
        
        // Status indexes
        if item.checked {
            self.checked_set.insert(item.id.clone());
        }
        if item.pinned {
            self.pinned_set.insert(item.id.clone());
        }
    }
    
    /// Remove item from indexes (O(1) amortized)
    fn remove_from_indexes(&mut self, item: &Arc<ItemModel>) {
        // Project index
        if let Some(project_id) = &item.project_id {
            if let Some(items) = self.project_index.get_mut(project_id) {
                items.retain(|i| i.id != item.id);
                if items.is_empty() {
                    self.project_index.remove(project_id);
                }
            }
        }
        
        // Section index
        if let Some(section_id) = &item.section_id {
            if let Some(items) = self.section_index.get_mut(section_id) {
                items.retain(|i| i.id != item.id);
                if items.is_empty() {
                    self.section_index.remove(section_id);
                }
            }
        }
        
        // Status indexes
        self.checked_set.remove(&item.id);
        self.pinned_set.remove(&item.id);
    }
    
    /// Update item in indexes (handles property changes)
    fn update_item_in_indexes(&mut self, old_item: &Arc<ItemModel>, new_item: &Arc<ItemModel>) {
        // Remove old, add new
        self.remove_from_indexes(old_item);
        self.add_to_indexes(new_item);
    }
}
```

#### Modified Mutation Methods

```rust
impl TodoStore {
    pub fn add_item(&mut self, item: Arc<ItemModel>) {
        self.all_items.push(item.clone());
        self.add_to_indexes(&item);
        self.increment_version();
    }
    
    pub fn update_item(&mut self, item: Arc<ItemModel>) {
        if let Some(pos) = self.all_items.iter().position(|i| i.id == item.id) {
            let old_item = self.all_items[pos].clone();
            self.all_items[pos] = item.clone();
            self.update_item_in_indexes(&old_item, &item);
            self.increment_version();
        }
    }
    
    pub fn remove_item(&mut self, id: &str) {
        if let Some(pos) = self.all_items.iter().position(|i| i.id == id) {
            let item = self.all_items.remove(pos);
            self.remove_from_indexes(&item);
            self.increment_version();
        }
    }
    
    // Similar updates for projects, sections, labels
    pub fn add_project(&mut self, project: Arc<ProjectModel>) {
        self.projects.push(project);
        self.increment_version();
    }
    
    pub fn update_project(&mut self, project: Arc<ProjectModel>) {
        if let Some(pos) = self.projects.iter().position(|p| p.id == project.id) {
            self.projects[pos] = project;
            self.increment_version();
        }
    }
    
    pub fn remove_project(&mut self, id: &str) {
        self.projects.retain(|p| p.id != id);
        self.increment_version();
    }
}
```

### 2. Event System

```rust
/// Fine-grained events for TodoStore changes
#[derive(Debug, Clone)]
pub enum TodoStoreEvent {
    // Item events
    ItemAdded(String),           // item_id
    ItemUpdated(String),         // item_id
    ItemDeleted(String),         // item_id
    ItemChecked(String, bool),   // item_id, checked
    ItemPinned(String, bool),    // item_id, pinned
    
    // Project events
    ProjectAdded(String),        // project_id
    ProjectUpdated(String),      // project_id
    ProjectDeleted(String),      // project_id
    
    // Section events
    SectionAdded(String),        // section_id
    SectionUpdated(String),      // section_id
    SectionDeleted(String),      // section_id
    
    // Label events
    LabelAdded(String),          // label_id
    LabelUpdated(String),        // label_id
    LabelDeleted(String),        // label_id
    
    // Bulk operations
    BulkUpdate,                  // Multiple changes at once
}

impl TodoStore {
    /// Emit an event (for future use with event bus)
    fn emit_event(&self, event: TodoStoreEvent) {
        // For Phase 1, this is a placeholder
        // Phase 2 will implement full event bus
        tracing::debug!("TodoStore event: {:?}", event);
    }
}
```

### 3. Board View Optimization

```rust
pub struct InboxBoard {
    base: BoardBase,
    cached_version: usize,  // NEW: Cache TodoStore version
}

impl InboxBoard {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut base = BoardBase::new(window, cx);
        
        base._subscriptions = vec![
            cx.observe_global_in::<TodoStore>(window, move |this, window, cx| {
                let store = cx.global::<TodoStore>();
                
                // NEW: Version check - skip if unchanged
                if this.cached_version == store.version() {
                    return;  // No changes, skip recomputation
                }
                
                // Update cached version
                this.cached_version = store.version();
                
                // Recompute view data
                this.update_view(store, window, cx);
                cx.notify();
            }),
        ];
        
        Self { 
            base,
            cached_version: 0,  // Initialize to 0
        }
    }
    
    fn update_view(&mut self, store: &TodoStore, window: &mut Window, cx: &mut Context<Self>) {
        let state_items = store.inbox_items();
        
        // Update item_rows
        self.base.item_rows = state_items
            .iter()
            .filter(|item| !item.checked)
            .map(|item| cx.new(|cx| ItemRowState::new(item.clone(), window, cx)))
            .collect();
        
        // Regroup items
        self.base.no_section_items.clear();
        self.base.section_items_map.clear();
        self.base.pinned_items.clear();
        
        for (i, item) in state_items.iter().enumerate() {
            if !item.checked {
                if item.pinned {
                    self.base.pinned_items.push((i, item.clone()));
                } else {
                    match item.section_id.as_deref() {
                        None | Some("") => {
                            self.base.no_section_items.push((i, item.clone()))
                        },
                        Some(sid) => {
                            self.base
                                .section_items_map
                                .entry(sid.to_string())
                                .or_default()
                                .push((i, item.clone()));
                        },
                    }
                }
            }
        }
        
        // Update active index
        if let Some(ix) = self.base.active_index {
            if ix >= self.base.item_rows.len() {
                self.base.active_index =
                    if self.base.item_rows.is_empty() { None } else { Some(0) };
            }
        } else if !self.base.item_rows.is_empty() {
            self.base.active_index = Some(0);
        }
    }
}
```

### 4. Database Connection Management

```rust
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use gpui::Global;

/// Optimized database state with Arc wrapper
pub struct DBState {
    pub conn: Arc<DatabaseConnection>,  // Changed from DatabaseConnection to Arc
}

impl Global for DBState {}

impl DBState {
    /// Create DBState from a DatabaseConnection
    pub fn new(conn: DatabaseConnection) -> Self {
        Self {
            conn: Arc::new(conn),
        }
    }
    
    /// Get a cloneable reference to the connection
    pub fn connection(&self) -> Arc<DatabaseConnection> {
        Arc::clone(&self.conn)
    }
}

// Usage in async operations
pub fn add_item(item: Arc<ItemModel>, cx: &mut App) {
    let db = cx.global::<DBState>().connection();  // Clone Arc, not connection
    
    cx.spawn(async move |cx| {
        // Use (*db) to dereference Arc when needed
        match Store::new((*db).clone()).insert_item(item).await {
            Ok(new_item) => {
                cx.update_global::<TodoStore>(|store, _| {
                    store.add_item(Arc::new(new_item));
                });
            }
            Err(e) => {
                tracing::error!("Failed to add item: {:?}", e);
            }
        }
    }).detach();
}
```

### 5. Performance Monitoring

```rust
use std::time::Instant;
use tracing::{debug, warn, info};

/// Performance monitoring utility
pub struct PerformanceMonitor;

impl PerformanceMonitor {
    /// Track operation timing and log if slow
    pub fn track_operation<F, R>(name: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();
        
        let duration_ms = duration.as_millis();
        
        if duration_ms > 100 {
            warn!(
                operation = name,
                duration_ms = duration_ms,
                "Slow operation detected"
            );
        } else {
            debug!(
                operation = name,
                duration_ms = duration_ms,
                "Operation completed"
            );
        }
        
        result
    }
    
    /// Track async operation timing
    pub async fn track_async_operation<F, Fut, R>(name: &str, f: F) -> R
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = R>,
    {
        let start = Instant::now();
        let result = f().await;
        let duration = start.elapsed();
        
        let duration_ms = duration.as_millis();
        
        if duration_ms > 100 {
            warn!(
                operation = name,
                duration_ms = duration_ms,
                "Slow async operation detected"
            );
        } else {
            debug!(
                operation = name,
                duration_ms = duration_ms,
                "Async operation completed"
            );
        }
        
        result
    }
}

// Usage examples
impl TodoStore {
    pub fn inbox_items(&self) -> Vec<Arc<ItemModel>> {
        PerformanceMonitor::track_operation("inbox_items", || {
            self.all_items
                .iter()
                .filter(|item| {
                    !item.checked
                        && (item.project_id.is_none() || item.project_id.as_deref() == Some(""))
                })
                .cloned()
                .collect()
        })
    }
}
```

## Data Models

### TodoStore State

```rust
pub struct TodoStore {
    // Core data (unchanged)
    pub all_items: Vec<Arc<ItemModel>>,
    pub projects: Vec<Arc<ProjectModel>>,
    pub labels: Vec<Arc<LabelModel>>,
    pub sections: Vec<Arc<SectionModel>>,
    pub active_project: Option<Arc<ProjectModel>>,
    
    // Indexes (unchanged structure, optimized updates)
    project_index: HashMap<String, Vec<Arc<ItemModel>>>,
    section_index: HashMap<String, Vec<Arc<ItemModel>>>,
    checked_set: HashSet<String>,
    pinned_set: HashSet<String>,
    
    // NEW: Version tracking
    version: usize,
}
```

### Board View State

```rust
pub struct InboxBoard {
    base: BoardBase,
    cached_version: usize,  // NEW
}

pub struct TodayBoard {
    base: BoardBase,
    cached_version: usize,  // NEW
}

pub struct ScheduledBoard {
    base: BoardBase,
    cached_version: usize,  // NEW
}

pub struct ProjectBoard {
    base: BoardBase,
    cached_version: usize,  // NEW
}
```

### Database State

```rust
pub struct DBState {
    pub conn: Arc<DatabaseConnection>,  // Changed from DatabaseConnection
}
```

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*


### Property Reflection

After analyzing all acceptance criteria, I identified several opportunities to consolidate redundant properties:

**Consolidations:**
- Requirements 1.4 and 1.5 (project_id and section_id changes) can be combined into a single property about index updates when any indexed field changes
- Requirements 1.6 and 1.7 (checked and pinned status changes) can be combined into a single property about status set updates
- Requirements 2.2, 2.3, 2.4, 2.5 (version increments for different entity types) can be combined into a single property about version increments on any mutation
- Requirements 7.1, 7.2, 7.3 (timing for add/update/delete) can be combined into a single property about mutation operation timing
- Requirements 7.5 and 7.6 (query timing for different views) can be combined into a single property about query operation timing

**Retained as separate:**
- Requirement 1.8 (index consistency) is the master invariant that subsumes 1.1-1.7 but we keep specific properties for targeted testing
- Requirement 3.3 and 3.4 (version check behavior) are complementary cases that should be tested separately
- Performance benchmarks (Requirement 7) are kept separate for different operation types to enable targeted optimization

### Correctness Properties

Property 1: Index Consistency Invariant
*For any* TodoStore and any sequence of operations (add, update, delete), the indexes (project_index, section_index, checked_set, pinned_set) SHALL always match the state that would result from calling rebuild_indexes() on the current all_items
**Validates: Requirements 1.8**

Property 2: Incremental Index Updates
*For any* task operation (add, update, delete), the TodoStore SHALL update only the affected index entries without calling rebuild_indexes()
**Validates: Requirements 1.1, 1.2, 1.3**

Property 3: Indexed Field Changes
*For any* task with indexed fields (project_id, section_id, checked, pinned), when any indexed field changes, the TodoStore SHALL update only the indexes affected by that specific field change
**Validates: Requirements 1.4, 1.5, 1.6, 1.7**

Property 4: Version Increment on Mutation
*For any* TodoStore mutation operation (add, update, delete of items, projects, sections, or labels), the version counter SHALL increment by exactly 1
**Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5**

Property 5: Version Monotonicity
*For any* sequence of TodoStore operations, the version number SHALL be monotonically increasing (allowing for wrapping at usize::MAX)
**Validates: Requirements 2.7**

Property 6: Observer Version Caching
*For any* Board view observing TodoStore, when the cached_version equals TodoStore.version(), the Board SHALL skip all recomputation
**Validates: Requirements 3.1, 3.2, 3.3**

Property 7: Observer Version Update
*For any* Board view observing TodoStore, when the cached_version differs from TodoStore.version(), the Board SHALL update cached_version and recompute its view data
**Validates: Requirements 3.4**

Property 8: Observer Batching
*For any* sequence of N TodoStore mutations occurring within a single event loop tick, Board views SHALL re-render at most once
**Validates: Requirements 3.7**

Property 9: Event Emission
*For any* TodoStore mutation operation, the appropriate TodoStoreEvent variant SHALL be emitted with minimal data (IDs only)
**Validates: Requirements 4.7, 4.8**

Property 10: Arc Connection Sharing
*For any* database operation, the system SHALL clone Arc<DatabaseConnection> pointers rather than cloning the underlying DatabaseConnection
**Validates: Requirements 5.1, 5.2**

Property 11: Arc Reference Counting
*For any* async database operation, when the operation completes, the Arc<DatabaseConnection> reference count SHALL decrement automatically
**Validates: Requirements 5.5**

Property 12: Performance Monitoring Thresholds
*For any* monitored operation, if execution time exceeds 100ms, a warning SHALL be logged; otherwise, debug information SHALL be logged
**Validates: Requirements 6.3, 6.4**

Property 13: Mutation Operation Performance
*For any* task mutation operation (add, update, delete) on a TodoStore with N tasks, the operation SHALL complete in less than 50ms regardless of N
**Validates: Requirements 7.1, 7.2, 7.3**

Property 14: Query Operation Performance
*For any* TodoStore query operation (inbox_items, today_items, etc.) on a TodoStore with 1000 tasks, the operation SHALL complete in less than 10ms
**Validates: Requirements 7.5, 7.6**

Property 15: View Rendering Performance
*For any* Board view switch operation, the view SHALL render in less than 100ms
**Validates: Requirements 7.4**

Property 16: Scalability
*For any* TodoStore containing 10,000 tasks, all mutation and query operations SHALL still meet their respective performance targets
**Validates: Requirements 7.8**

Property 17: API Compatibility
*For any* existing public method on TodoStore, the method signature and behavior SHALL remain unchanged after optimization
**Validates: Requirements 8.1, 8.5**

Property 18: Data Integrity
*For any* task data loaded from existing database, all properties and relationships SHALL be preserved after optimization
**Validates: Requirements 8.3, 8.4**

Property 19: Edge Case Handling
*For any* task with edge case values (null project_id, empty section_id, missing fields), the TodoStore SHALL handle the task correctly without errors
**Validates: Requirements 8.7**

## Error Handling

### Index Update Errors

The incremental index update system is designed to be infallible for valid inputs. However, we handle edge cases:

```rust
impl TodoStore {
    fn add_to_indexes(&mut self, item: &Arc<ItemModel>) {
        // Defensive: Check for empty IDs before indexing
        if let Some(project_id) = &item.project_id {
            if !project_id.is_empty() {
                self.project_index
                    .entry(project_id.clone())
                    .or_default()
                    .push(item.clone());
            } else {
                tracing::warn!(
                    item_id = %item.id,
                    "Task has empty project_id, skipping project index"
                );
            }
        }
        
        // Similar checks for section_id
        if let Some(section_id) = &item.section_id {
            if !section_id.is_empty() {
                self.section_index
                    .entry(section_id.clone())
                    .or_default()
                    .push(item.clone());
            }
        }
    }
}
```

### Version Overflow

Version counter uses wrapping arithmetic to handle overflow gracefully:

```rust
impl TodoStore {
    fn increment_version(&mut self) {
        self.version = self.version.wrapping_add(1);
        
        if self.version == 0 {
            // Wrapped around - log for monitoring
            tracing::info!("TodoStore version counter wrapped to 0");
        }
    }
}
```

### Database Connection Errors

Database operations should handle connection errors gracefully:

```rust
pub async fn add_item_with_error_handling(
    item: Arc<ItemModel>,
    cx: &mut App,
) -> Result<(), TodoError> {
    let db = cx.global::<DBState>().connection();
    
    match Store::new((*db).clone()).insert_item(item.clone()).await {
        Ok(new_item) => {
            cx.update_global::<TodoStore>(|store, _| {
                store.add_item(Arc::new(new_item));
            });
            Ok(())
        }
        Err(e) => {
            tracing::error!(
                item_id = %item.id,
                error = %e,
                "Failed to add item to database"
            );
            Err(e)
        }
    }
}
```

### Performance Monitoring Errors

Performance monitoring should never fail the operation being monitored:

```rust
impl PerformanceMonitor {
    pub fn track_operation<F, R>(name: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        
        // Execute operation - any panic here is from the operation, not monitoring
        let result = f();
        
        // Monitoring code should not panic
        let duration = start.elapsed();
        let duration_ms = duration.as_millis();
        
        // Use defensive logging that won't panic
        if duration_ms > 100 {
            let _ = std::panic::catch_unwind(|| {
                warn!(
                    operation = name,
                    duration_ms = duration_ms,
                    "Slow operation detected"
                );
            });
        }
        
        result
    }
}
```

## Testing Strategy

### Dual Testing Approach

We will use both unit tests and property-based tests for comprehensive coverage:

**Unit Tests**: Verify specific examples, edge cases, and error conditions
- Specific scenarios (empty store, single item, multiple items)
- Edge cases (null values, empty strings, boundary conditions)
- Error conditions (invalid data, connection failures)
- Integration points between components

**Property Tests**: Verify universal properties across all inputs
- Index consistency across random operation sequences
- Version tracking behavior with random mutations
- Performance characteristics with varying data sizes
- Observer optimization with random state changes

Both approaches are complementary and necessary for comprehensive coverage.

### Property-Based Testing Configuration

We will use the `proptest` crate for property-based testing:

```toml
[dev-dependencies]
proptest = "1.4"
```

Each property test will:
- Run minimum 100 iterations (due to randomization)
- Reference its design document property in a comment
- Use the tag format: **Feature: phase-1-performance-optimization, Property {number}: {property_text}**

### Test Organization

```
crates/mytool/src/todo_state/
├── todo_store.rs
├── todo_store_tests.rs          # Unit tests
└── todo_store_properties.rs     # Property-based tests

crates/mytool/src/views/boards/
├── board_inbox.rs
├── board_inbox_tests.rs         # Unit tests
└── board_properties.rs          # Property-based tests (shared)

crates/mytool/src/performance/
├── monitor.rs
└── monitor_tests.rs             # Unit tests
```

### Key Test Scenarios

#### 1. Index Consistency Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    
    // Feature: phase-1-performance-optimization, Property 1: Index Consistency Invariant
    proptest! {
        #[test]
        fn index_consistency_after_operations(
            operations in prop::collection::vec(arbitrary_operation(), 1..100)
        ) {
            let mut store = TodoStore::new();
            
            // Apply random operations
            for op in operations {
                apply_operation(&mut store, op);
            }
            
            // Verify indexes match rebuild
            let mut expected_store = store.clone();
            expected_store.rebuild_indexes();
            
            assert_eq!(store.project_index, expected_store.project_index);
            assert_eq!(store.section_index, expected_store.section_index);
            assert_eq!(store.checked_set, expected_store.checked_set);
            assert_eq!(store.pinned_set, expected_store.pinned_set);
        }
    }
}
```

#### 2. Version Tracking Tests

```rust
// Feature: phase-1-performance-optimization, Property 4: Version Increment on Mutation
proptest! {
    #[test]
    fn version_increments_on_mutation(item in arbitrary_item()) {
        let mut store = TodoStore::new();
        let initial_version = store.version();
        
        store.add_item(Arc::new(item));
        
        assert_eq!(store.version(), initial_version + 1);
    }
}
```

#### 3. Observer Optimization Tests

```rust
// Feature: phase-1-performance-optimization, Property 6: Observer Version Caching
#[test]
fn observer_skips_recompute_when_version_unchanged() {
    let mut store = TodoStore::new();
    let mut board = InboxBoard::new_for_test();
    
    // Set matching versions
    board.cached_version = store.version();
    
    // Track if update_view was called
    let update_called = Arc::new(AtomicBool::new(false));
    let update_called_clone = update_called.clone();
    
    board.set_update_callback(move || {
        update_called_clone.store(true, Ordering::SeqCst);
    });
    
    // Trigger observer without changing store
    board.on_store_change(&store);
    
    // Verify update was skipped
    assert!(!update_called.load(Ordering::SeqCst));
}
```

#### 4. Performance Benchmark Tests

```rust
// Feature: phase-1-performance-optimization, Property 13: Mutation Operation Performance
#[test]
fn add_item_performance() {
    let mut store = TodoStore::new();
    
    // Add 1000 items to establish baseline
    for i in 0..1000 {
        store.add_item(Arc::new(create_test_item(&format!("item_{}", i))));
    }
    
    // Measure add operation
    let start = Instant::now();
    store.add_item(Arc::new(create_test_item("new_item")));
    let duration = start.elapsed();
    
    assert!(duration.as_millis() < 50, "add_item took {}ms", duration.as_millis());
}

// Feature: phase-1-performance-optimization, Property 14: Query Operation Performance
#[test]
fn inbox_items_performance() {
    let mut store = TodoStore::new();
    
    // Add 1000 items
    for i in 0..1000 {
        store.add_item(Arc::new(create_test_item(&format!("item_{}", i))));
    }
    
    // Measure query operation
    let start = Instant::now();
    let _ = store.inbox_items();
    let duration = start.elapsed();
    
    assert!(duration.as_millis() < 10, "inbox_items took {}ms", duration.as_millis());
}
```

#### 5. Backward Compatibility Tests

```rust
// Feature: phase-1-performance-optimization, Property 17: API Compatibility
#[test]
fn api_compatibility() {
    let mut store = TodoStore::new();
    
    // Verify all existing methods still work
    let item = Arc::new(create_test_item("test"));
    
    // These should all compile and work
    store.add_item(item.clone());
    store.update_item(item.clone());
    store.remove_item("test");
    
    let _ = store.inbox_items();
    let _ = store.today_items();
    let _ = store.scheduled_items();
    let _ = store.completed_items();
    let _ = store.pinned_items();
    let _ = store.items_by_project("project1");
    let _ = store.items_by_section("section1");
}

// Feature: phase-1-performance-optimization, Property 19: Edge Case Handling
proptest! {
    #[test]
    fn handles_edge_cases(
        project_id in prop::option(prop::string::string_regex(".*").unwrap()),
        section_id in prop::option(prop::string::string_regex(".*").unwrap())
    ) {
        let mut store = TodoStore::new();
        
        let item = ItemModel {
            id: "test".to_string(),
            project_id,
            section_id,
            ..Default::default()
        };
        
        // Should not panic
        store.add_item(Arc::new(item));
        
        // Indexes should be consistent
        let mut expected = store.clone();
        expected.rebuild_indexes();
        assert_eq!(store.project_index, expected.project_index);
    }
}
```

### Test Coverage Goals

- **Unit test coverage**: 80%+ for modified components
- **Property test coverage**: All 19 correctness properties
- **Performance test coverage**: All timing requirements (7.1-7.8)
- **Integration test coverage**: End-to-end workflows with optimized components

### Continuous Integration

Tests will run on every commit:
```yaml
# .github/workflows/test.yml
- name: Run unit tests
  run: cargo test --lib
  
- name: Run property tests
  run: cargo test --test properties -- --test-threads=1
  
- name: Run performance benchmarks
  run: cargo bench --bench performance
```

## Implementation Notes

### Phase 1 Scope

This design focuses exclusively on Phase 1 optimizations:
- ✅ Incremental index updates
- ✅ Version tracking
- ✅ Observer optimization
- ✅ Database connection management
- ✅ Performance monitoring

Out of scope for Phase 1:
- ❌ Full event bus implementation (placeholder only)
- ❌ Cache layer for query results
- ❌ Batch operation optimization
- ❌ Offline support
- ❌ Advanced performance profiling

### Migration Strategy

The optimization is designed for zero-downtime deployment:

1. **Add new fields** (version, cached_version) with default values
2. **Add new methods** (increment_version, add_to_indexes, etc.)
3. **Update existing methods** to use new infrastructure
4. **Keep rebuild_indexes** as fallback for bulk operations
5. **Deploy incrementally** with feature flags if needed

### Performance Expectations

Based on the optimization plan analysis:

| Metric | Current | Target | Expected Improvement |
|--------|---------|--------|---------------------|
| Index update | O(n) | O(1) | 50%+ faster |
| View re-renders | 100% | 30% | 70% reduction |
| Task add/update | Variable | <50ms | Consistent |
| View switching | Variable | <100ms | Consistent |
| Memory usage | Stable | Stable | No regression |

### Monitoring and Validation

Post-deployment monitoring:
- Track operation timing via PerformanceMonitor logs
- Monitor version counter growth rate
- Track observer notification frequency
- Measure memory usage over time
- Collect user-reported performance feedback

## Dependencies

### Existing Dependencies

```toml
[dependencies]
gpui = { git = "https://github.com/zed-industries/zed" }
sea-orm = { version = "1.1", features = ["sqlx-sqlite"] }
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

### New Dependencies

```toml
[dev-dependencies]
proptest = "1.4"  # For property-based testing
criterion = "0.5"  # For performance benchmarking
```

### Version Requirements

- Rust: 2024 edition (already in use)
- GPUI: Latest from zed repository
- SeaORM: 1.1+
- Tokio: 1.0+

## Future Enhancements

While out of scope for Phase 1, these enhancements build on this foundation:

### Phase 2: Event Bus and Cache Layer
- Full event bus implementation with subscriptions
- Query result caching with automatic invalidation
- Fine-grained observer subscriptions

### Phase 3: Batch Operations
- Batch insert/update/delete operations
- Transaction support for atomic changes
- Optimized bulk data loading

### Phase 4: Advanced Performance
- Connection pooling for database
- Background index maintenance
- Predictive preloading
- Memory usage optimization

## Conclusion

This design provides a solid foundation for Phase 1 performance optimization while maintaining backward compatibility and enabling future enhancements. The incremental approach minimizes risk while delivering measurable performance improvements.

Key success metrics:
- ✅ 50%+ improvement in index update performance
- ✅ 70% reduction in unnecessary view re-renders
- ✅ Consistent sub-50ms task operation timing
- ✅ Comprehensive test coverage (80%+)
- ✅ Zero data migration required
- ✅ Full backward compatibility maintained
