PRAGMA foreign_keys = ON;
CREATE TABLE IF NOT EXISTS links(
    id INTEGER PRIMARY KEY NOT NULL,
    link TEXT NOT NULL,
    title TEXT NOT NULL,
    UNIQUE(link, title)
);
CREATE INDEX IF NOT EXISTS idx_links_link ON links(link);