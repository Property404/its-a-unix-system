use crate::process::Process;
use anyhow::Result;
use futures::io::AsyncWriteExt;

pub async fn cowsay(process: &mut Process, args: Vec<String>) -> Result<()> {
    // TODO: Read entire input
    let text = if args.len() == 1 {
        process.stdin.get_line().await?
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
