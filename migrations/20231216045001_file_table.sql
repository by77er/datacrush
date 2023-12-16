create table files (
  id serial primary key,
  location text not null,
  size_bytes bigint not null,
  content_type text not null,
  created_at timestamp not null default now(),
  downloads integer not null default 0
);

create unique index files_location_uindex on files (location);