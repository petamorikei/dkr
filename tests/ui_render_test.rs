use ratatui::backend::TestBackend;
use ratatui::Terminal;

#[test]
fn test_backend_creation() {
    let backend = TestBackend::new(80, 24);
    let terminal = Terminal::new(backend).unwrap();
    
    let size = terminal.size().unwrap();
    assert_eq!(size.width, 80);
    assert_eq!(size.height, 24);
}