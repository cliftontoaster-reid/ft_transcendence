ALTER TABLE users
ADD COLUMN username varchar(64) UNIQUE,
ADD COLUMN bio varchar(500),
ADD COLUMN age_bracket integer;