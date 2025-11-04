CREATE TABLE IF NOT EXISTS key_chains(
	key_name	TEXT PRIMARY KEY NOT NULL
,	uses		SMALLINT NOT NULL
,	entry_level	SMALLINT REFERENCES user_levels(level_id) NOT NULL
);

-- trigger function to remove keys which are used up
CREATE FUNCTION delete_key_chain_on_used_up()
RETURNS trigger AS $$
BEGIN
	IF NEW.uses <= 0 THEN
		DELETE FROM key_chains WHERE key_name = NEW.key_name;
		RETURN NULL;
	END IF;
	RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- apply trigger
CREATE TRIGGER auto_delete_kay_chain_on_used_up
BEFORE UPDATE ON key_chains
FOR EACH ROW
EXECUTE FUNCTION delete_key_chain_on_used_up();

INSERT INTO key_chains
	(key_name, uses, entry_level)
SELECT
	'dev', 1, level_id
FROM
	user_levels
WHERE
	level_name = 'admin'
;