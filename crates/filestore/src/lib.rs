use std::{io::Error, path::{Path, PathBuf}};

use bytes::Bytes;

use futures_core::stream::Stream;
use tokio::{fs::{ OpenOptions, create_dir_all }, io::{AsyncWriteExt, AsyncReadExt}};
use tokio_util::io::{ReaderStream, StreamReader};

#[derive(Clone)]
pub struct FileStore {
    base_path: String,
}

impl FileStore {
    pub fn new(base_path: String) -> Self {
        Self {
            base_path
        }
    }

    fn get_path(&self, path: &Path) -> Result<PathBuf, Error> {
        println!("path: {:?}", path);
        Ok(Path::new(&self.base_path).join(path))
    }

    pub async fn create_file<T: Into<std::io::Error>>(&mut self, path: &Path, stream: impl Stream<Item = Result<Bytes, T>>) -> Result<usize, Error> {
        let path = self.get_path(path)?;
        if let Some(dir) = Path::new(&path).parent() {
            create_dir_all(dir).await?;
        }
        let mut file = OpenOptions::new()
            .create_new(true)
            .create(true)
            .write(true)
            .open(path)
            .await?;

        let mut stream = Box::pin(StreamReader::new(stream));
        let mut chunk = [0; 64];
        let mut byte_count: usize = 0;
        loop {
            let bytes = stream.read(&mut chunk).await?;
            if bytes == 0 {
                break;
            }
            byte_count += bytes;
            file.write_all(&chunk[..bytes]).await?;
        }
        Ok(byte_count)
    }
    
    pub async fn delete_file(&mut self, path: &Path) -> Result<(), Error> {
        tokio::fs::remove_file(self.get_path(path)?).await?;
        Ok(())
    }
    
    pub async fn get_file(&self, path: &Path) -> Result<impl Stream<Item = Result<Bytes, Error>>, Error> {
        let file = tokio::fs::File::open(self.get_path(path)?).await?;
        Ok(ReaderStream::new(file))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util;
    use bytes::Bytes;


    #[tokio::test]
    async fn test_create_file() {
        let mut filestore = FileStore::new("objects".to_string());
        let stream = futures_util::stream::once(async { Ok::<_, Error>(Bytes::from("Hello, World!")) });
        let result = filestore.create_file("prefix/test.txt", stream).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_file() {
        let mut filestore = FileStore::new("objects".to_string());
        let stream = futures_util::stream::once(async { Ok::<_, Error>(Bytes::from("Hello, World!")) });
        let result = filestore.create_file("prefix/test.txt", stream).await;
        assert!(result.is_ok());
        let stream = filestore.get_file("prefix/test.txt").await;
        assert!(stream.is_ok());
        let mut stream = StreamReader::new(stream.unwrap());
        let mut chunk = [0; 64];
        let bytes = stream.read(&mut chunk).await.unwrap();
        assert_eq!(bytes, 13);
        assert_eq!(&chunk[..bytes], b"Hello, World!");
    }

    #[tokio::test]
    async fn test_delete_file() {
        let mut filestore = FileStore::new("objects".to_string());
        let stream = futures_util::stream::once(async { Ok::<_, Error>(Bytes::from("Hello, World!")) });
        let result = filestore.create_file("prefix/test.txt", stream).await;
        assert!(result.is_ok());
        let result = filestore.delete_file("prefix/test.txt").await;
        assert!(result.is_ok());
        let stream = filestore.get_file("prefix/test.txt").await;
        assert!(stream.is_err());
    }
}