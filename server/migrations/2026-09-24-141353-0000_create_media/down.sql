-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS "media";
DROP TABLE IF EXISTS "media_translations";
DROP TYPE IF EXISTS "media_image_type";
DROP TABLE IF EXISTS "media_images";
DROP TABLE IF EXISTS "media_images";
DROP INDEX IF EXISTS "media_images_media_id_idx";