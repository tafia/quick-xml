use tokio::io::{AsyncWrite, AsyncWriteExt};

use crate::errors::Result;
use crate::events::Event;
use crate::Writer;

impl<W: AsyncWrite + Unpin> Writer<W> {
    /// Writes the given event to the underlying writer. Async version of [Writer::write_event].
    pub async fn write_event_async<'a, E: AsRef<Event<'a>>>(&mut self, event: E) -> Result<()> {
        match *event.as_ref() {
            Event::Start(ref e) => self.write_wrapped_async(b"<", e, b">").await,
            Event::End(ref e) => self.write_wrapped_async(b"</", e, b">").await,
            Event::Empty(ref e) => self.write_wrapped_async(b"<", e, b"/>").await,
            Event::Text(ref e) => self.write_async(e).await,
            Event::Comment(ref e) => self.write_wrapped_async(b"<!--", e, b"-->").await,
            Event::CData(ref e) => {
                self.write_async(b"<![CDATA[").await?;
                self.write_async(e).await?;
                self.write_async(b"]]>").await
            }
            Event::Decl(ref e) => self.write_wrapped_async(b"<?", e, b"?>").await,
            Event::PI(ref e) => self.write_wrapped_async(b"<?", e, b"?>").await,
            Event::DocType(ref e) => self.write_wrapped_async(b"<!DOCTYPE ", e, b">").await,
            Event::Eof => Ok(()),
        }
    }

    #[inline]
    async fn write_async(&mut self, value: &[u8]) -> Result<()> {
        self.writer.write_all(value).await.map_err(Into::into)
    }

    #[inline]
    async fn write_wrapped_async(
        &mut self,
        before: &[u8],
        value: &[u8],
        after: &[u8],
    ) -> Result<()> {
        self.write_async(before).await?;
        self.write_async(value).await?;
        self.write_async(after).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::*;
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn xml_header() {
        let mut buffer = Vec::new();
        let mut writer = Writer::new(&mut buffer);

        let event = Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), Some("no")));
        writer
            .write_event_async(event)
            .await
            .expect("write tag failed");

        assert_eq!(
            std::str::from_utf8(&buffer).unwrap(),
            r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?>"#
        );
    }
}
