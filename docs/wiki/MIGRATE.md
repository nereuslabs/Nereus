# Как перенести этот контент в GitHub Wiki

Эти страницы хранятся в `docs/wiki/` репозитория, пока настоящий Wiki не
провижионирован (GitHub требует одну первую правку через веб‑UI, чтобы
`.wiki.git`‑backend создался).

1. Один раз откройте веб‑Wiki и создайте первую страницу, чтобы GitHub
   создал `Nereus.wiki.git`:
   ```
   gh browse --wiki
   # в браузере нажмите "Create the first page" и сохраните Home.md
   ```
2. Клонируйте wiki‑репо и запушьте страницы:
   ```bash
   git clone https://github.com/Yan123-tech/Nereus.wiki.git /tmp/Nereus.wiki
   cp docs/wiki/*.md /tmp/Nereus.wiki/
   cd /tmp/Nereus.wiki && git add . && git commit -m "docs: import wiki pages" && git push
   ```
3. Готово — страницы доступны на `github.com/Yan123-tech/Nereus/wiki`.
