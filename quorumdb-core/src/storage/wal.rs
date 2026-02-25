use std::path::PathBuf;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum LogEntry {
    Set { key: String, value: String },
    Delete { key: String },
}

#[allow(dead_code)]
pub struct WriteAheadLog {
    file: Option<File>,
    path: PathBuf,
}

impl WriteAheadLog {
    pub async fn new(path: &str) -> Result<Self, std::io::Error> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;

        Ok(Self {
            file: Some(file),
            path: PathBuf::from(path),
        })
    }

    pub async fn write(&mut self, entry: &LogEntry) -> Result<(), String> {
        let file = self.file.as_mut()
            .ok_or("Write-ahead log file is not initialized")?;

        // Serialize to MessagePack bytes
        let data = rmp_serde::to_vec(entry)
            .map_err(|e| format!("Serialization error: {}", e))?;

        // Write length prefix (4 bytes, big-endian)
        let len = data.len() as u32;
        file.write_all(&len.to_be_bytes()).await
            .map_err(|e| format!("Write length error: {}", e))?;

        // Write MessagePack data
        file.write_all(&data).await
            .map_err(|e| format!("Write error: {}", e))?;

        file.sync_all().await
            .map_err(|e| format!("Sync error: {}", e))?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn replay(&self) -> Result<Vec<LogEntry>, Box<dyn std::error::Error>> {
        match File::open(&self.path).await {
            Ok(mut file) => {
                let mut entries = Vec::new();
                let mut len_buf = [0u8; 4];

                loop {
                    // Read length prefix
                    match file.read_exact(&mut len_buf).await {
                        Ok(_) => {
                            let len = u32::from_be_bytes(len_buf) as usize;

                            // Read MessagePack data
                            let mut data = vec![0u8; len];
                            file.read_exact(&mut data).await?;

                            // Deserialize
                            let entry: LogEntry = rmp_serde::from_slice(&data)?;
                            entries.push(entry);
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                            break; // End of file
                        }
                        Err(e) => return Err(Box::new(e)),
                    }
                }
                Ok(entries)
            }
            Err(_) => Ok(Vec::new()),
        }
    }
}
