use crate::process::Process;
use anyhow::Result;
use futures::io::{AsyncReadExt, AsyncWriteExt};

pub async fn cowsay(process: &mut Process, args: Vec<String>) -> Result<()> {
    let text = if args.len() == 1 {
        let mut text = String::new();
        process.stdin.read_to_string(&mut text).await?;
        text
    } else {
        args.into_iter().skip(1).collect::<Vec<_>>().join(" ")
    };
    let text = format!(
        "
  ____
 < {text} >
  ----
     \\    ^__^
      \\   (oo)\\_______
          (__)\\       )\\/\\ 
              ||----w |
              ||     ||
"
    );
    process.stdout.write_all(text.as_bytes()).await?;
    Ok(())
}
