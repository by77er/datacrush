create table pastes (
  id serial primary key,
  data text not null,
  slug text not null,
  created_at timestamp not null default now(),
  views integer not null default 0
);

create unique index pastes_slug_idx on pastes(slug);