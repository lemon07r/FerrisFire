use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, MenuId},
    TrayIcon, TrayIconBuilder,
};

pub struct SystemTray {
    _tray_icon: TrayIcon,
    pub show_item_id: MenuId,
    pub quit_item_id: MenuId,
}

impl SystemTray {
    pub fn new() -> Option<Self> {
        let icon = load_tray_icon()?;

        let menu = Menu::new();
        let show_item = MenuItem::new("Show", true, None);
        let quit_item = MenuItem::new("Quit", true, None);
        
        let show_item_id = show_item.id().clone();
        let quit_item_id = quit_item.id().clone();

        menu.append(&show_item).ok()?;
        menu.append(&quit_item).ok()?;

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("FerrisFire")
            .with_icon(icon)
            .build()
            .ok()?;

        Some(Self {
            _tray_icon: tray_icon,
            show_item_id,
            quit_item_id,
        })
    }

    pub fn poll_events(&self) -> Option<TrayEvent> {
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.show_item_id {
                return Some(TrayEvent::Show);
            } else if event.id == self.quit_item_id {
                return Some(TrayEvent::Quit);
            }
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayEvent {
    Show,
    Quit,
}

fn load_tray_icon() -> Option<tray_icon::Icon> {
    let icon_bytes = include_bytes!("../assets/ferrisfire.ico");
    
    let img = image::load_from_memory(icon_bytes).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    
    tray_icon::Icon::from_rgba(rgba.into_raw(), width, height).ok()
}
