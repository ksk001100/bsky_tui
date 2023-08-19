use chrono::{DateTime, FixedOffset};

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

pub fn get_duration_string(t1: DateTime<FixedOffset>, t2: DateTime<FixedOffset>) -> String {
    let duration = t2 - t1;

    if duration.num_weeks() != 0 {
        format!("{}w", duration.num_weeks())
    } else if duration.num_days() != 0 {
        format!("{}d", duration.num_days())
    } else if duration.num_hours() != 0 {
        format!("{}h", duration.num_hours())
    } else if duration.num_minutes() != 0 {
        format!("{}m", duration.num_minutes())
    } else if duration.num_seconds() != 0 {
        format!("{}s", duration.num_seconds())
    } else {
        format!("{}ms", duration.num_milliseconds())
    }
}

pub fn get_splash(path: Option<String>) -> String {
    if let Some(path) = path {
        std::fs::read_to_string(path).unwrap_or_else(|_| SPLASH.to_string())
    } else {
        SPLASH.to_string()
    }
}
