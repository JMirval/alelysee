-- Rename auth + storage columns to provider-agnostic names.
do $$ begin
    if exists (
        select 1
        from information_schema.columns
        where table_name = 'users' and column_name = 'cognito_sub'
    ) then
        alter table users rename column cognito_sub to auth_subject;
    end if;
end $$;

do $$ begin
    if exists (
        select 1
        from information_schema.columns
        where table_name = 'videos' and column_name = 's3_bucket'
    ) then
        alter table videos rename column s3_bucket to storage_bucket;
    end if;
end $$;

do $$ begin
    if exists (
        select 1
        from information_schema.columns
        where table_name = 'videos' and column_name = 's3_key'
    ) then
        alter table videos rename column s3_key to storage_key;
    end if;
end $$;
