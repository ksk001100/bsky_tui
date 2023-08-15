pub mod app;
pub mod bsky;
pub mod inputs;
pub mod io;
pub mod utils;

use std::{io::stdout, sync::Arc, time::Duration};

use eyre::Result;
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::{
    app::{ui, App, AppReturn},
    inputs::{events::Events, InputEvent},
    io::IoEvent,
};

// Bluesky splash screen
const SPLASH: &str = r#"


                                                                
             ,,                                                 
`7MM"""Yp, `7MM                            `7MM                 
  MM    Yb   MM                              MM                 
  MM    dP   MM `7MM  `7MM  .gP"Ya  ,pP"Ybd  MM  ,MP'`7M'   `MF'
  MM"""bg.   MM   MM    MM ,M'   Yb 8I   `"  MM ;Y     VA   ,V  
  MM    `Y   MM   MM    MM 8M"""""" `YMMMa.  MM;Mm      VA ,V   
  MM    ,9   MM   MM    MM YM.    , L.   I8  MM `Mb.     VVV    
.JMMmmmd9  .JMML. `Mbod"YML.`Mbmmd' M9mmmP'.JMML. YA.    ,V     
                                                        ,V      
                                                     OOb"       
"#;

pub async fn start_ui(app: &Arc<tokio::sync::Mutex<App>>) -> Result<()> {
    let stdout = stdout();
    crossterm::terminal::enable_raw_mode()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    terminal.hide_cursor()?;
    // terminal.show_cursor()?;

    let tick_rate = Duration::from_millis(200);
    let mut events = Events::new(tick_rate);

    {
        let mut app = app.lock().await;
        app.dispatch(IoEvent::Initialize).await;
    }

    let mut split_splash: Vec<String> = SPLASH.split('\n').map(|s| s.to_string()).collect();
    for i in 0..split_splash.len() {
        terminal.draw(|rect| ui::render_splash(rect, split_splash.join("\n")))?;
        if i == 0 {
            tokio::time::sleep(Duration::from_millis(1000)).await;
        } else {
            tokio::time::sleep(Duration::from_millis(70)).await;
        }
        split_splash.pop();
    }

    loop {
        let mut app = app.lock().await;

        terminal.draw(|rect| ui::render(rect, &app))?;

        let result = match events.next().await {
            InputEvent::Input(key) => app.do_action(key).await,
            InputEvent::Tick => app.update_on_tick().await,
        };

        if result == AppReturn::Exit {
            events.close();
            break;
        }
    }

    terminal.clear()?;
    terminal.show_cursor()?;
    crossterm::terminal::disable_raw_mode()?;

    Ok(())
}
