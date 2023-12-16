create table urls (
  id serial primary key,
  url text not null,
  slug text not null,
  created_at timestamp not null default now(),
  uses integer not null default 0
);

create unique index urls_slug_idx on urls(slug);