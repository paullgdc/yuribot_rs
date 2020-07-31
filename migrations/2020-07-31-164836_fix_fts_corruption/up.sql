-- Drop previous full text index and rebuild it --
DROP TABLE links_title_idx;
DROP TRIGGER links_fts_ai;
DROP TRIGGER links_fts_ad;
DROP TRIGGER links_fts_au;

CREATE VIRTUAL TABLE links_title_idx USING fts5(id UNINDEXED, link UNINDEXED, title, content='links', content_rowid='id');
INSERT INTO links_title_idx(rowid, title) SELECT id, title FROM links;
CREATE TRIGGER links_fts_ai AFTER INSERT ON links BEGIN
  INSERT INTO links_title_idx(rowid,title) VALUES (new.id, new.title);
END;
CREATE TRIGGER links_fts_ad AFTER DELETE ON links BEGIN
  INSERT INTO links_title_idx(links_title_idx, rowid, title) VALUES('delete', old.id, old.title);
END;
CREATE TRIGGER links_fts_au AFTER UPDATE ON links BEGIN
  INSERT INTO links_title_idx(links_title_idx, rowid, title) VALUES('delete', old.id, old.title);
  INSERT INTO links_title_idx(rowid,title) VALUES (new.id, new.title);
END;
