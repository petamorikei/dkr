# TUI Testing Approaches for dkr

## 1. Specialized TUI Testing Frameworks

### **insta** (Snapshot Testing)
```toml
[dev-dependencies]
insta = "1.34"
```
- Terminal output のスナップショットテスト
- UIの回帰テストに最適
- 変更の視覚的な確認が可能

### **ratatui-testkit** 
```rust
use ratatui::backend::TestBackend;
use ratatui::Terminal;
```
- Ratatui組み込みのテストバックエンド
- バッファの内容を文字列として検証可能

### **expectrl** (PTY-based Testing)
```toml
[dev-dependencies]
expectrl = "0.7"
```
- 実際のターミナルエミュレーション
- キー入力と出力の検証
- 最もリアルなE2Eテスト

## 2. Framework-Free Testing Approaches

### **A. Mock Docker Backend**
```rust
// tests/mock_docker.rs
pub struct MockDockerClient {
    containers: Vec<ContainerSummary>,
    should_fail: bool,
}

impl MockDockerClient {
    pub async fn list_containers(&self, _all: bool) -> Result<Vec<ContainerSummary>> {
        if self.should_fail {
            Err(anyhow!("Mock connection error"))
        } else {
            Ok(self.containers.clone())
        }
    }
}
```

### **B. TestBackend with Assertions**
```rust
#[cfg(test)]
mod tests {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use crate::ui::render;

    #[test]
    fn test_container_list_rendering() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        
        let mut app = create_test_app();
        let containers = vec![
            create_test_container("nginx", "Running"),
            create_test_container("redis", "Stopped"),
        ];
        
        terminal.draw(|f| {
            render(f, &mut app, &containers, &vec![], &None, &vec![]).unwrap();
        }).unwrap();
        
        let buffer = terminal.backend().buffer();
        
        // Check if container names are rendered
        assert!(buffer_contains(buffer, "nginx"));
        assert!(buffer_contains(buffer, "redis"));
        assert!(buffer_contains(buffer, "Running"));
        assert!(buffer_contains(buffer, "Stopped"));
    }
    
    fn buffer_contains(buffer: &ratatui::buffer::Buffer, text: &str) -> bool {
        let content = buffer_to_string(buffer);
        content.contains(text)
    }
    
    fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
        let mut lines = vec![];
        for y in 0..buffer.area.height {
            let mut line = String::new();
            for x in 0..buffer.area.width {
                let cell = buffer.get(x, y);
                line.push_str(cell.symbol());
            }
            lines.push(line.trim_end().to_string());
        }
        lines.join("\n")
    }
}
```

### **C. Integration Tests with Script**
```bash
#!/bin/bash
# tests/integration_test.sh

# Start test containers
docker run -d --name test-nginx nginx:alpine
docker run -d --name test-redis redis:alpine

# Run dkr with scripted inputs
timeout 5 bash -c '
    echo -e "j\nj\ns\nq" | cargo run 2>&1 | tee test_output.txt
'

# Check output
if grep -q "test-nginx" test_output.txt && grep -q "test-redis" test_output.txt; then
    echo "✓ Containers listed correctly"
else
    echo "✗ Failed to list containers"
    exit 1
fi

# Cleanup
docker rm -f test-nginx test-redis
```

### **D. State Machine Testing**
```rust
#[cfg(test)]
mod state_tests {
    use crate::app::{App, AppTab};
    use crate::event::Action;

    #[tokio::test]
    async fn test_navigation() {
        let mut app = App::new().await.unwrap();
        
        assert_eq!(app.current_tab, AppTab::Containers);
        assert_eq!(app.selected_index, 0);
        
        // Test tab navigation
        app.next_tab();
        assert_eq!(app.current_tab, AppTab::Images);
        
        app.previous_tab();
        assert_eq!(app.current_tab, AppTab::Containers);
        
        // Test list navigation
        app.select_next(10);
        assert_eq!(app.selected_index, 1);
        
        app.select_previous();
        assert_eq!(app.selected_index, 0);
    }
    
    #[tokio::test]
    async fn test_keyboard_shortcuts() {
        let mut app = App::new().await.unwrap();
        
        // Simulate pressing '2'
        app.current_tab = AppTab::Images;
        assert_eq!(app.current_tab, AppTab::Images);
        
        // Test quit
        app.quit();
        assert!(app.should_quit);
    }
}
```

## 3. Recommended Testing Strategy for dkr

### **Level 1: Unit Tests** (Fast, Isolated)
```rust
// src/docker/container.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_container_state_from_str() {
        assert_eq!(ContainerState::from_str("running"), ContainerState::Running);
        assert_eq!(ContainerState::from_str("exited"), ContainerState::Exited);
        assert_eq!(ContainerState::from_str("unknown"), ContainerState::Unknown);
    }
}
```

### **Level 2: Component Tests** (TestBackend)
```rust
// tests/ui_tests.rs
#[test]
fn test_error_popup_display() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let mut app = create_test_app();
    app.set_error("Test error message".to_string());
    
    terminal.draw(|f| {
        render(f, &mut app, &vec![], &vec![], &None, &vec![]).unwrap();
    }).unwrap();
    
    let buffer = terminal.backend().buffer();
    assert!(buffer_contains(buffer, "Test error message"));
    assert!(buffer_contains(buffer, "Error"));
}
```

### **Level 3: Integration Tests** (Mock Docker)
```rust
// tests/integration.rs
#[tokio::test]
async fn test_container_operations() {
    let mock_docker = Arc::new(Mutex::new(MockDockerClient::new()));
    let mut app = App::with_docker(mock_docker.clone()).await.unwrap();
    
    // Add test container
    mock_docker.lock().await.add_container(
        create_test_container("test-1", "Running")
    );
    
    // Test stop operation
    let containers = mock_docker.lock().await.list_containers(true).await.unwrap();
    assert_eq!(containers[0].state, ContainerState::Running);
    
    // Simulate stop
    mock_docker.lock().await.stop_container("test-1").await.unwrap();
    
    let containers = mock_docker.lock().await.list_containers(true).await.unwrap();
    assert_eq!(containers[0].state, ContainerState::Exited);
}
```

### **Level 4: Snapshot Tests** (insta)
```rust
// tests/snapshots.rs
use insta::assert_snapshot;

#[test]
fn test_ui_snapshot() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    
    let app = create_standard_test_app();
    terminal.draw(|f| {
        render(f, &mut app, &test_containers(), &vec![], &None, &vec![]).unwrap();
    }).unwrap();
    
    let output = buffer_to_string(terminal.backend().buffer());
    assert_snapshot!(output);
}
```

## 4. CI/CD Integration

### GitHub Actions Example
```yaml
# .github/workflows/test.yml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    
    services:
      docker:
        image: docker:dind
        options: --privileged
    
    steps:
    - uses: actions/checkout@v3
    
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
    
    - name: Run Unit Tests
      run: cargo test --lib
    
    - name: Run Integration Tests  
      run: cargo test --test '*'
    
    - name: Run TUI Snapshot Tests
      run: cargo insta test
    
    - name: Run E2E Tests with Docker
      run: |
        docker run -d --name test-container nginx:alpine
        cargo test --features integration
        docker rm -f test-container
```

## 5. Test Helpers

```rust
// tests/helpers/mod.rs
pub fn create_test_app() -> App {
    App {
        config: Config::default(),
        docker: Arc::new(Mutex::new(MockDockerClient::new())),
        current_tab: AppTab::Containers,
        selected_index: 0,
        should_quit: false,
        show_help: false,
        show_logs: false,
        show_inspect: false,
        log_viewer: None,
        inspect_viewer: None,
        error_message: None,
    }
}

pub fn create_test_container(name: &str, status: &str) -> ContainerSummary {
    ContainerSummary {
        id: format!("{}-id", name),
        name: name.to_string(),
        image: "test:latest".to_string(),
        status: status.to_string(),
        state: ContainerState::from_str(status),
        created: 1234567890,
        ports: vec![],
    }
}
```

## Recommended Approach for dkr

1. **Unit Tests**: データモデルとビジネスロジック
2. **TestBackend Tests**: UI表示の正確性
3. **Mock Docker Tests**: Docker操作のシミュレーション  
4. **Snapshot Tests**: UIの回帰テスト
5. **Script-based E2E**: 実環境での動作確認（CI用）

この組み合わせで、高速なフィードバックループと信頼性の高いテストカバレッジを実現できます。