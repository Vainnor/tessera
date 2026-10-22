# Privacy principles

Tessera's notes belong to the people who write them. These are project rules, not settings, and
a change that breaks one is not merged. The website's privacy policy
(`apps/web/src/pages/privacy.astro`) is the user-facing version and is a draft pending legal
review.

1. **No telemetry.** No analytics, usage statistics, crash reports, "anonymous" metrics or
   pings, in the app, the sync server or the website. Not opt-in, not opt-out: none.
2. **Local first.** Notes are plain files on your device and the app works fully offline. Nothing
   leaves the device unless you set up sync.
3. **End-to-end encrypted sync** (planned). The server will store and relay only ciphertext. It
   will still see some metadata, such as account ids, IP addresses, sizes and timing; the full
   list is in [`docs/threat-model.md`](docs/threat-model.md#what-the-server-inevitably-sees).
4. **No third parties.** No CDNs, web fonts, trackers, embeds or advertising, in the app or on
   the website.
5. **No accounts or cookies on the website.** The only thing it stores is your theme choice, in
   your own browser.
6. **Logs hold no personal data.** No note content, file names, paths or IP addresses at the
   default log level.
7. **Self-hosting is first class.** You can run your own sync server under the same license.
