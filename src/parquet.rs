use crate::DataSource;
use arrow::record_batch::RecordBatch;
use parquet::arrow::arrow_reader::RowSelector;
use parquet::file::reader::FileReader;
use parquet::file::serialized_reader::SerializedFileReader;
use std::fs::File;
use std::time::Instant;

pub struct ParquetFile {
    file: File,
    n_rows: usize,
}

impl ParquetFile {
    pub fn new(file: File) -> anyhow::Result<ParquetFile> {
        // We don't support live-updating parquet files, so we may as well cache
        // the row count
        let n_rows = count_rows(&file)?;
        Ok(ParquetFile { file, n_rows })
    }
}

impl DataSource for ParquetFile {
    fn row_count(&self) -> anyhow::Result<usize> {
        Ok(self.n_rows)
    }

    fn fetch_batch(&self, offset: usize, len: usize) -> anyhow::Result<RecordBatch> {
        let file = self.file.try_clone()?;
        let mut rdr = parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder::try_new(file)?
            .with_batch_size(len)
            .with_row_selection(
                vec![
                    RowSelector {
                        row_count: offset,
                        skip: true,
                    },
                    RowSelector {
                        row_count: len,
                        skip: false,
                    },
                ]
                .into(),
            )
            .build()?;
        let batch = rdr.next().unwrap()?;
        Ok(batch)
    }
}

fn count_rows(file: &File) -> anyhow::Result<usize> {
    let start = Instant::now();
    let file = file.try_clone()?;
    let rdr = SerializedFileReader::new(file)?;
    let total_rows = rdr.metadata().file_metadata().num_rows() as usize;
    eprintln!("Counted {total_rows} rows (took {:?})", start.elapsed());
    Ok(total_rows)
}
