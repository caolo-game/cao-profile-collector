
--  CREATE OR REPLACE FUNCTION on_record_update() RETURNS trigger
--  AS $$
--  BEGIN
--      NEW.updated := now();
--      RETURN NEW;
--  END;
--  $$
--  LANGUAGE plpgsql;

DROP TRIGGER record_updated ON record;

CREATE TRIGGER record_updated BEFORE UPDATE ON record
FOR EACH ROW EXECUTE PROCEDURE on_record_update();
