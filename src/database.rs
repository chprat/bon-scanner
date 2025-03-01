pub struct Database {
    connection: sqlite::Connection,
}

impl Database {
    pub fn create_bon(&self, date: &str, price: f64) {
        let query = format!("INSERT INTO bons (date, price) VALUES ('{date}', '{price}')");
        self.connection.execute(query).expect("Couldn't insert bon");
    }

    pub fn create_category(&self, category: &str) {
        let query = format!("INSERT INTO categories (category) VALUES ('{category}')");
        self.connection
            .execute(query)
            .expect("Couldn't insert category");
    }

    pub fn create_database(&self) {
        let query = "
            CREATE TABLE bons (bonId INTEGER PRIMARY KEY AUTOINCREMENT, date TEXT NOT NULL, price REAL NOT NULL);
            CREATE TABLE categories (categoryId INTEGER PRIMARY KEY AUTOINCREMENT, category TEXT NOT NULL);
            CREATE TABLE entries (entryId INTEGER PRIMARY KEY AUTOINCREMENT, bonId INTEGER NOT NULL, productId INTEGER NOT NULL, price REAL NOT NULL);
            CREATE TABLE products (productId INTEGER PRIMARY KEY AUTOINCREMENT, categoryId INTEGER NOT NULL, product TEXT NOT NULL);
        ";
        self.connection
            .execute(query)
            .expect("Couldn't create database");
    }

    pub fn create_entry(&self, bon_id: i64, product_id: i64, price: f64) {
        let query = format!(
            "INSERT INTO entries (bonId, productId, price) VALUES ('{bon_id}', '{product_id}', '{price}')"
        );
        self.connection
            .execute(query)
            .expect("Couldn't insert entry");
    }

    pub fn create_product(&self, category_id: i64, product: &str) {
        let query = format!(
            "INSERT INTO products (categoryId, product) VALUES ('{category_id}', '{product}')"
        );
        self.connection
            .execute(query)
            .expect("Couldn't insert product");
    }

    pub fn get_bons(&self) -> Vec<Bon> {
        let mut empty_bons: Vec<Bon> = Vec::new();
        let query = "SELECT * FROM bons";
        for row in self
            .connection
            .prepare(query)
            .expect("Couldn't prepare statement")
            .into_iter()
            .map(|row| row.expect("Couldn't fetch row"))
        {
            let bon_id = row.read::<i64, _>("bonId");
            let bon_date = row.read::<&str, _>("date");
            let bon_price = row.read::<f64, _>("price");
            let mut bon = Bon::new(bon_date, bon_price);
            bon.bon_id = bon_id;
            empty_bons.push(bon);
        }
        let mut bons: Vec<Bon> = Vec::new();
        for empty_bon in empty_bons {
            let mut bon = Bon::new(&empty_bon.date, empty_bon.price);
            let bon_id = empty_bon.bon_id;
            let query = format!(
                "SELECT category, price, product FROM entries e
                 JOIN products USING (productId)
                 JOIN categories USING (categoryId)
                 WHERE bonId = '{bon_id}'"
            );
            for row in self
                .connection
                .prepare(query)
                .expect("Couldn't prepare statement")
                .into_iter()
                .map(|row| row.expect("Couldn't fetch row"))
            {
                let entry_category = row.read::<&str, _>("category");
                let entry_price = row.read::<f64, _>("price");
                let entry_product = row.read::<&str, _>("product");
                let entry = Entry::new(entry_category, entry_product, entry_price);
                bon.entries.push(entry);
            }
            bons.push(bon);
        }
        bons
    }

    pub fn new(database_file: &str) -> Self {
        Self {
            connection: sqlite::open(database_file).expect("Couldn't open database"),
        }
    }
}

#[derive(Debug)]
pub struct Bon {
    bon_id: i64,
    pub date: String,
    pub price: f64,
    pub entries: Vec<Entry>,
}

impl Bon {
    pub fn new(date: &str, price: f64) -> Self {
        Self {
            bon_id: 0,
            date: date.to_string(),
            price,
            entries: Vec::new(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Entry {
    pub category: String,
    pub product: String,
    pub price: f64,
}

impl Entry {
    pub fn new(category: &str, product: &str, price: f64) -> Self {
        Self {
            category: category.to_string(),
            product: product.to_string(),
            price,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlite::State;

    #[test]
    fn create_database() {
        let mut tables: Vec<String> = Vec::new();
        let query = "SELECT name FROM sqlite_master WHERE type='table'";
        let database = Database::new(":memory:");
        database.create_database();
        database
            .connection
            .iterate(query, |pairs| {
                for &(_name, value) in pairs.iter() {
                    tables.push(value.expect("Values is not available").to_string());
                }
                true
            })
            .expect("Couldn't execute query");
        assert_eq!(5, tables.len());
        assert!(tables.contains(&"bons".to_string()));
        assert!(tables.contains(&"categories".to_string()));
        assert!(tables.contains(&"entries".to_string()));
        assert!(tables.contains(&"products".to_string()));
        assert!(tables.contains(&"sqlite_sequence".to_string()));
    }

    #[test]
    fn create_bon() {
        let query = "SELECT date, price FROM bons";
        let database = Database::new(":memory:");
        database.create_database();
        database.create_bon("2024-12-24 12:12:12 +0100", 25.47);
        let mut statement = database
            .connection
            .prepare(query)
            .expect("Couldn't prepare statement");

        while let Ok(State::Row) = statement.next() {
            let date = statement
                .read::<String, _>("date")
                .expect("Couldn't read date");
            let price = statement
                .read::<f64, _>("price")
                .expect("Couldn't read price");
            assert_eq!("2024-12-24 12:12:12 +0100", date);
            assert_eq!(25.47, price);
        }
    }

    #[test]
    fn create_category() {
        let query = "SELECT category FROM categories";
        let database = Database::new(":memory:");
        database.create_database();
        database.create_category("food");
        for row in database
            .connection
            .prepare(query)
            .expect("Couldn't prepare statement")
            .into_iter()
            .map(|row| row.expect("Couldn't fetch row"))
        {
            assert_eq!("food", row.read::<&str, _>("category"));
        }
    }

    #[test]
    fn create_entry() {
        let query = "SELECT bonId, productId, price FROM entries";
        let database = Database::new(":memory:");
        database.create_database();
        database.create_entry(1, 1, 2.99);
        let mut statement = database
            .connection
            .prepare(query)
            .expect("Couldn't prepare statement");

        while let Ok(State::Row) = statement.next() {
            let bon = statement
                .read::<i64, _>("bonId")
                .expect("Couldn't read bon");
            let product = statement
                .read::<i64, _>("productId")
                .expect("Couldn't read product");
            let price = statement
                .read::<f64, _>("price")
                .expect("Couldn't read price");
            assert_eq!(1, bon);
            assert_eq!(1, product);
            assert_eq!(2.99, price);
        }
    }

    #[test]
    fn create_product() {
        let query = "SELECT categoryId, product FROM products";
        let database = Database::new(":memory:");
        database.create_database();
        database.create_product(1, "butter");
        for row in database
            .connection
            .prepare(query)
            .expect("Couldn't prepare statement")
            .into_iter()
            .map(|row| row.expect("Couldn't fetch row"))
        {
            assert_eq!(1, row.read::<i64, _>("categoryId"));
            assert_eq!("butter", row.read::<&str, _>("product"));
        }
    }

    #[test]
    fn get_bons() {
        let database = Database::new(":memory:");
        database.create_database();
        database.create_bon("2024-12-24 12:12:12 +0100", 25.47);
        database.create_bon("2024-12-25 13:12:12 +0100", 26.47);
        database.create_category("food");
        database.create_category("stuff");
        database.create_product(1, "butter");
        database.create_product(1, "eggs");
        database.create_product(2, "spoon");
        database.create_product(2, "fork");
        database.create_entry(1, 1, 2.99);
        database.create_entry(1, 2, 3.99);
        database.create_entry(2, 2, 3.49);
        database.create_entry(2, 3, 4.99);
        database.create_entry(2, 4, 5.99);

        let butter = Entry::new("food", "butter", 2.99);
        let eggs1 = Entry::new("food", "eggs", 3.99);
        let eggs2 = Entry::new("food", "eggs", 3.49);
        let spoon = Entry::new("stuff", "spoon", 4.99);
        let fork = Entry::new("stuff", "fork", 5.99);

        let bons = database.get_bons();
        assert_eq!(2, bons.len());
        let bon = &bons[0];
        assert_eq!("2024-12-24 12:12:12 +0100", bon.date);
        assert_eq!(25.47, bon.price);
        assert_eq!(2, bon.entries.len());
        assert!(bon.entries.contains(&butter));
        assert!(bon.entries.contains(&eggs1));

        let bon = &bons[1];
        assert_eq!("2024-12-25 13:12:12 +0100", bon.date);
        assert_eq!(26.47, bon.price);
        assert_eq!(3, bon.entries.len());
        assert!(bon.entries.contains(&eggs2));
        assert!(bon.entries.contains(&spoon));
        assert!(bon.entries.contains(&fork));
    }
}
