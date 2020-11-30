use std::path::Path;
use std::{io, fs};
use crate::database::{Datastore, Column};

pub fn read_csv(path: &Path, name: &str) -> io::Result<Datastore> {
    let file = fs::read_to_string(path)?;
    let split = file
        .split("\n")
        .filter(|line| !line.is_empty())
        .map(|line| line.replace("\r", ""))
        .map(|line| {
            line.split(",")
                .map(|x| x.to_owned())
                .collect::<Vec<String>>()
        })
        .collect::<Vec<_>>();

    let header = match split.first() {
        None => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("No header in CSV: {}", path.display()),
            ));
        }
        Some(v) => v,
    };

    if header.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Empty header in CSV: {}", path.display()),
        ));
    }

    let data = split.iter().skip(1).collect::<Vec<_>>();

    let mut columns = header
        .into_iter()
        .map(|h| Column {
            name: (*h).to_owned(),
            data: vec![],
        })
        .collect::<Vec<_>>();

    for row in data {
        if row.len() != header.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Inconsistent amount of data in row: {:?} (should have: {} columns, has: {})",
                    row,
                    header.len(),
                    row.len()
                ),
            ));
        }
        for i in 0..row.len() {
            let val = (*row.get(i).unwrap()).to_owned();
            columns.get_mut(i).unwrap().data.push(val);
        }
    }

    Ok(Datastore {
        path: path.to_owned(),
        name: name.to_owned(),
        columns,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_csv_with_2_columns() {
        let tempdir = tempfile::tempdir().unwrap();
        let csv_path = tempdir.path().join("data.csv");
        std::fs::write(&csv_path, "col1,col2\n1,2\n2,3").unwrap();

        let datastore = read_csv(&csv_path, "data").unwrap();
        assert_eq!(datastore.columns.len(), 2);

        assert_eq!(datastore.columns.get(0).unwrap().name, "col1");
        assert_eq!(datastore.columns.get(0).unwrap().data.len(), 2);
        assert_eq!(datastore.columns.get(0).unwrap().data.get(0).unwrap(), "1");
        assert_eq!(datastore.columns.get(0).unwrap().data.get(1).unwrap(), "2");

        assert_eq!(datastore.columns.get(1).unwrap().name, "col2");
        assert_eq!(datastore.columns.get(1).unwrap().data.len(), 2);
        assert_eq!(datastore.columns.get(1).unwrap().data.get(0).unwrap(), "2");
        assert_eq!(datastore.columns.get(1).unwrap().data.get(1).unwrap(), "3");
    }
}