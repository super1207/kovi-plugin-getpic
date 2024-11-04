use kovi::Message;
use kovi::PluginBuilder as plugin;
use kovi::log;
use regex::Regex;
use reqwest::header::HeaderMap;
use kovi::tokio;

use base64::{Engine as _, engine::{self, general_purpose}, alphabet};
const BASE64_CUSTOM_ENGINE: engine::GeneralPurpose = engine::GeneralPurpose::new(&alphabet::STANDARD, general_purpose::PAD);

const PLUS_NAME:&str = "GETPIC";


fn get_random() -> Result<usize, getrandom::Error> {
    let mut rand_buf = [0u8; std::mem::size_of::<usize>()];
    getrandom::getrandom(&mut rand_buf)?;
    let mut num = 0usize;
    for i in 0..std::mem::size_of::<usize>() {
        num = (num << 8) + (rand_buf[i] as usize);
    }
    Ok(num)
}

fn substr(s: &str, start: usize, length: usize) -> String {
    s.chars().skip(start).take(length).collect()
}

// 函数返回关键词和图片的base64
fn deal_str0(str0:&str) -> Result<(String,String), Box<dyn std::error::Error>> {
    let key_word = substr(str0,3,str0.chars().count() - 6);
    log::debug!("[{PLUS_NAME}]key_word:{key_word}，正在构造链接...");
    let mut headers = HeaderMap::new();
    let client = reqwest::blocking::Client::new();
    headers.insert("User-Agent", "Mozilla/5.0 (Windows NT 6.1; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/89.0.4389.72 Safari/537.36".parse()?);
    headers.insert("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9".parse()?);
    let url = "https://image.baidu.com/search/index?tn=baiduimage&word=".to_owned() + &key_word.as_str();
    let http_ret = client.get(url.as_str()).headers(headers).send()?.text()?;
    let http_str = http_ret.as_str();
    let re = Regex::new("\"objURL\" {0,1}: {0,1}\"(.*?)\"")?;
    let mut cap_vec = Vec::new();
    for cap in re.captures_iter(http_str) {
        let retstr = cap[1].to_string();
        cap_vec.push(retstr);
    }
    let index = get_random()? % cap_vec.len();
    let pic_url = cap_vec.get(index).ok_or("vec index range out")?;
    let image_buffer = client.get(pic_url).send()?.bytes()?;
    let b64_str = BASE64_CUSTOM_ENGINE.encode(image_buffer);
    return Ok((key_word,b64_str));
}


fn need_deal(str0:&str) -> bool {
    if !str0.starts_with("#来点") || !str0.ends_with("的图片") {
        return false;
    }
    return true;
}

#[kovi::plugin]
async fn main() {
    plugin::on_msg(|event| async move {

        // 取文本
        let str0_opt = event.borrow_text();
        let str0_ref;
        if let Some(str0) = str0_opt {
            str0_ref = str0;
        } else {
            return;
        }

        // 判断是否需要处理
        if !need_deal(str0_ref) {
            return;
        }

        // 拷贝一份
        let str0 = str0_ref.to_string();
        
        // 得到涩涩处理结果
        let deal_ret_rst = tokio::task::spawn_blocking(move || {
            match deal_str0(&str0) {
                Ok(ret) => {
                   return Some(ret);
                },
                Err(err) => {
                    log::error!("[{PLUS_NAME}] error:{err}");
                    return None;
                }
            }
        }).await;

        // 不知道在干嘛，反正这样就编译过了
        let deal_ret = match deal_ret_rst {
            Ok(deal_ret_t) => {
                deal_ret_t
            },
            Err(err) => {
                log::error!("[{PLUS_NAME}] error:{err}");
                None
            },
        };

        // 发出去！
        if let Some((key,b64_url)) = deal_ret {
            let msg = Message::new();
            let msg = msg.add_reply(event.message_id);
            let msg = msg.add_text(format!("喵喵喵，{key}的图片来啦~\r\n"));
            let msg = msg.add_text(format!("powered by kovi"));
            let msg = msg.add_image(&format!("base64://{b64_url}"));
            event.reply(msg);
        }

    });
}
