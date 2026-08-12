# О синхронизации `docs/wiki/` и GitHub Wiki

Эти страницы (`docs/wiki/*.md`) хранятся в репозитории как **зеркало** и теперь
**полностью синхронизированы** с настоящим GitHub Wiki по адресу
`github.com/nereuslabs/Nereus/wiki` (ветка `master` в `Nereus.wiki.git`).

Миграция выполнена: GitHub Wiki provisioned, страницы запушены. Поддерживать
согласованность лучше так: редактировать `Nereus.wiki` (он — canonical), а затем
скопировать страницы обратно в `docs/wiki/` и запушить в репо PR‑ом — так страницы
внутри репо не расходятся с публичным Wiki.

```bash
# выгрузить актуальный GitHub Wiki локально
git clone https://github.com/nereuslabs/Nereus.wiki.git /tmp/Nereus.wiki
cp /tmp/Nereus.wiki/*.md docs/wiki/
git add docs/wiki && git commit -m "docs: sync docs/wiki/ with GitHub Wiki" && git push
```
