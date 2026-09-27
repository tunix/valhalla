# Changelog

## [0.5.0](https://github.com/tunix/valhalla/compare/v0.4.0...v0.5.0) (2026-09-27)


### Features

* adopt horizon logo as official app icon ([5e6b84b](https://github.com/tunix/valhalla/commit/5e6b84b3ee359bcb7d411cb3689699cd01c0730b))
* cached HTTP client for image downloads ([4da4201](https://github.com/tunix/valhalla/commit/4da420171b5d2f854ee62a5636fe85afc76aff3a))
* CLI entry point with tokio runtime and --daemon mode ([7d0bbf7](https://github.com/tunix/valhalla/commit/7d0bbf7baf78639521548761f866f7bfa4f1d27e))
* **data:** flathub-ready appstream metadata ([#19](https://github.com/tunix/valhalla/issues/19)) ([ba0d587](https://github.com/tunix/valhalla/commit/ba0d587eddc83586bd20b1710b7990aa9894a93d))
* GSettings-backed config and GNOME Keyring credential storage ([26c419b](https://github.com/tunix/valhalla/commit/26c419bae73aa2c2dfd76f4352a99c71688a0851))
* libadwaita app shell with window and preferences ([bf6e6ec](https://github.com/tunix/valhalla/commit/bf6e6ecee5ecb8b44c6e28aaaba5b87f61eaf455))
* main window layout in Blueprint ([a3c6275](https://github.com/tunix/valhalla/commit/a3c6275d1e1c2ae064fe4aeed51eab1361cdc13e))
* **packaging:** publish .flatpak bundles with each release ([#18](https://github.com/tunix/valhalla/issues/18)) ([44fd425](https://github.com/tunix/valhalla/commit/44fd425fefffe2a2148f5a581aa9dce1ca45641e))
* perceptual brightness analysis for tone classification ([9333050](https://github.com/tunix/valhalla/commit/9333050cf19b76760c4e43cc037d45169f8c37fc))
* persisted state with cross-process refresh lock ([4a4b9c9](https://github.com/tunix/valhalla/commit/4a4b9c9a7706923d5c88245c618ffdf9dca3b6be))
* scheduler with theme matching, triggers and refresh timeout ([e56e9eb](https://github.com/tunix/valhalla/commit/e56e9ebfdfaa0a9cfd256820699e6220a4b2b437))
* screen probing and aspect-ratio mapping for wallpaper sources ([d94a8ee](https://github.com/tunix/valhalla/commit/d94a8eec29973a00ac4817f1b5d2d92ed09a8ccb))
* systemd user service management ([7673c9d](https://github.com/tunix/valhalla/commit/7673c9dfb795c6b456d87aec4c63c0b29c59e553))
* theme detection via XDG settings portal with gsettings fallback ([53b8589](https://github.com/tunix/valhalla/commit/53b85890d28dcebd43ccbd3a1fa8450ac4522b01))
* upscale wallpapers to cover the screen when smaller ([4272014](https://github.com/tunix/valhalla/commit/4272014f8ac48c68731c76e62d8c35968787289f))
* Wallhaven and Unsplash sources with tag, purity and orientation filters ([c6e6552](https://github.com/tunix/valhalla/commit/c6e6552ae2ca1dafe6bbf0e978a722679d8fcf5e))
* wallpaper application via portal and gsettings light/dark keys ([9bb2f9a](https://github.com/tunix/valhalla/commit/9bb2f9a2156149631036fc866da041a04397fe48))


### Bug Fixes

* **ci:** drop release-type input so release-please honors draft config ([fccf56c](https://github.com/tunix/valhalla/commit/fccf56c6fa1890ea853c5011a37280f76d5e1ac9))
* **ci:** let release-please honor draft config (immutable-release failure) ([#11](https://github.com/tunix/valhalla/issues/11)) ([fccf56c](https://github.com/tunix/valhalla/commit/fccf56c6fa1890ea853c5011a37280f76d5e1ac9))
* **ci:** move release-please config to repo root where the action looks ([#12](https://github.com/tunix/valhalla/issues/12)) ([e4af1fa](https://github.com/tunix/valhalla/commit/e4af1fa26d15fb224e89ef00eaf9c1ef2e4b41aa))
* **ci:** run the flatpak job in the flatpak-github-actions container ([#23](https://github.com/tunix/valhalla/issues/23)) ([d1d918d](https://github.com/tunix/valhalla/commit/d1d918d1ab3ac9f3d13c6945c838de891ae29eae))
* **ci:** skip systemd lifecycle test without user manager; make release publish idempotent ([#9](https://github.com/tunix/valhalla/issues/9)) ([142e0ce](https://github.com/tunix/valhalla/commit/142e0cef1a47305675418c9d7ea6d7ecefd59851))
* **release:** close the draft-mode duplicate-release race ([58b08e4](https://github.com/tunix/valhalla/commit/58b08e4d771bcf4ea5c59c4e715f729669214d82))
* **release:** close the draft-mode duplicate-release race (force-tag-creation) ([#24](https://github.com/tunix/valhalla/issues/24)) ([58b08e4](https://github.com/tunix/valhalla/commit/58b08e4d771bcf4ea5c59c4e715f729669214d82))

## [0.4.0](https://github.com/tunix/valhalla/compare/v0.3.0...v0.4.0) (2026-09-27)


### Features

* adopt horizon logo as official app icon ([5e6b84b](https://github.com/tunix/valhalla/commit/5e6b84b3ee359bcb7d411cb3689699cd01c0730b))
* cached HTTP client for image downloads ([4da4201](https://github.com/tunix/valhalla/commit/4da420171b5d2f854ee62a5636fe85afc76aff3a))
* CLI entry point with tokio runtime and --daemon mode ([7d0bbf7](https://github.com/tunix/valhalla/commit/7d0bbf7baf78639521548761f866f7bfa4f1d27e))
* **data:** flathub-ready appstream metadata ([#19](https://github.com/tunix/valhalla/issues/19)) ([ba0d587](https://github.com/tunix/valhalla/commit/ba0d587eddc83586bd20b1710b7990aa9894a93d))
* GSettings-backed config and GNOME Keyring credential storage ([26c419b](https://github.com/tunix/valhalla/commit/26c419bae73aa2c2dfd76f4352a99c71688a0851))
* libadwaita app shell with window and preferences ([bf6e6ec](https://github.com/tunix/valhalla/commit/bf6e6ecee5ecb8b44c6e28aaaba5b87f61eaf455))
* main window layout in Blueprint ([a3c6275](https://github.com/tunix/valhalla/commit/a3c6275d1e1c2ae064fe4aeed51eab1361cdc13e))
* **packaging:** publish .flatpak bundles with each release ([#18](https://github.com/tunix/valhalla/issues/18)) ([44fd425](https://github.com/tunix/valhalla/commit/44fd425fefffe2a2148f5a581aa9dce1ca45641e))
* perceptual brightness analysis for tone classification ([9333050](https://github.com/tunix/valhalla/commit/9333050cf19b76760c4e43cc037d45169f8c37fc))
* persisted state with cross-process refresh lock ([4a4b9c9](https://github.com/tunix/valhalla/commit/4a4b9c9a7706923d5c88245c618ffdf9dca3b6be))
* scheduler with theme matching, triggers and refresh timeout ([e56e9eb](https://github.com/tunix/valhalla/commit/e56e9ebfdfaa0a9cfd256820699e6220a4b2b437))
* screen probing and aspect-ratio mapping for wallpaper sources ([d94a8ee](https://github.com/tunix/valhalla/commit/d94a8eec29973a00ac4817f1b5d2d92ed09a8ccb))
* systemd user service management ([7673c9d](https://github.com/tunix/valhalla/commit/7673c9dfb795c6b456d87aec4c63c0b29c59e553))
* theme detection via XDG settings portal with gsettings fallback ([53b8589](https://github.com/tunix/valhalla/commit/53b85890d28dcebd43ccbd3a1fa8450ac4522b01))
* upscale wallpapers to cover the screen when smaller ([4272014](https://github.com/tunix/valhalla/commit/4272014f8ac48c68731c76e62d8c35968787289f))
* Wallhaven and Unsplash sources with tag, purity and orientation filters ([c6e6552](https://github.com/tunix/valhalla/commit/c6e6552ae2ca1dafe6bbf0e978a722679d8fcf5e))
* wallpaper application via portal and gsettings light/dark keys ([9bb2f9a](https://github.com/tunix/valhalla/commit/9bb2f9a2156149631036fc866da041a04397fe48))


### Bug Fixes

* **ci:** drop release-type input so release-please honors draft config ([fccf56c](https://github.com/tunix/valhalla/commit/fccf56c6fa1890ea853c5011a37280f76d5e1ac9))
* **ci:** let release-please honor draft config (immutable-release failure) ([#11](https://github.com/tunix/valhalla/issues/11)) ([fccf56c](https://github.com/tunix/valhalla/commit/fccf56c6fa1890ea853c5011a37280f76d5e1ac9))
* **ci:** move release-please config to repo root where the action looks ([#12](https://github.com/tunix/valhalla/issues/12)) ([e4af1fa](https://github.com/tunix/valhalla/commit/e4af1fa26d15fb224e89ef00eaf9c1ef2e4b41aa))
* **ci:** skip systemd lifecycle test without user manager; make release publish idempotent ([#9](https://github.com/tunix/valhalla/issues/9)) ([142e0ce](https://github.com/tunix/valhalla/commit/142e0cef1a47305675418c9d7ea6d7ecefd59851))

## [0.3.0](https://github.com/tunix/valhalla/compare/valhalla-v0.2.0...valhalla-v0.3.0) (2026-09-27)


### Features

* adopt horizon logo as official app icon ([5e6b84b](https://github.com/tunix/valhalla/commit/5e6b84b3ee359bcb7d411cb3689699cd01c0730b))
* cached HTTP client for image downloads ([4da4201](https://github.com/tunix/valhalla/commit/4da420171b5d2f854ee62a5636fe85afc76aff3a))
* CLI entry point with tokio runtime and --daemon mode ([7d0bbf7](https://github.com/tunix/valhalla/commit/7d0bbf7baf78639521548761f866f7bfa4f1d27e))
* GSettings-backed config and GNOME Keyring credential storage ([26c419b](https://github.com/tunix/valhalla/commit/26c419bae73aa2c2dfd76f4352a99c71688a0851))
* libadwaita app shell with window and preferences ([bf6e6ec](https://github.com/tunix/valhalla/commit/bf6e6ecee5ecb8b44c6e28aaaba5b87f61eaf455))
* main window layout in Blueprint ([a3c6275](https://github.com/tunix/valhalla/commit/a3c6275d1e1c2ae064fe4aeed51eab1361cdc13e))
* perceptual brightness analysis for tone classification ([9333050](https://github.com/tunix/valhalla/commit/9333050cf19b76760c4e43cc037d45169f8c37fc))
* persisted state with cross-process refresh lock ([4a4b9c9](https://github.com/tunix/valhalla/commit/4a4b9c9a7706923d5c88245c618ffdf9dca3b6be))
* scheduler with theme matching, triggers and refresh timeout ([e56e9eb](https://github.com/tunix/valhalla/commit/e56e9ebfdfaa0a9cfd256820699e6220a4b2b437))
* screen probing and aspect-ratio mapping for wallpaper sources ([d94a8ee](https://github.com/tunix/valhalla/commit/d94a8eec29973a00ac4817f1b5d2d92ed09a8ccb))
* systemd user service management ([7673c9d](https://github.com/tunix/valhalla/commit/7673c9dfb795c6b456d87aec4c63c0b29c59e553))
* theme detection via XDG settings portal with gsettings fallback ([53b8589](https://github.com/tunix/valhalla/commit/53b85890d28dcebd43ccbd3a1fa8450ac4522b01))
* upscale wallpapers to cover the screen when smaller ([4272014](https://github.com/tunix/valhalla/commit/4272014f8ac48c68731c76e62d8c35968787289f))
* Wallhaven and Unsplash sources with tag, purity and orientation filters ([c6e6552](https://github.com/tunix/valhalla/commit/c6e6552ae2ca1dafe6bbf0e978a722679d8fcf5e))
* wallpaper application via portal and gsettings light/dark keys ([9bb2f9a](https://github.com/tunix/valhalla/commit/9bb2f9a2156149631036fc866da041a04397fe48))


### Bug Fixes

* **ci:** drop release-type input so release-please honors draft config ([fccf56c](https://github.com/tunix/valhalla/commit/fccf56c6fa1890ea853c5011a37280f76d5e1ac9))
* **ci:** let release-please honor draft config (immutable-release failure) ([#11](https://github.com/tunix/valhalla/issues/11)) ([fccf56c](https://github.com/tunix/valhalla/commit/fccf56c6fa1890ea853c5011a37280f76d5e1ac9))
* **ci:** move release-please config to repo root where the action looks ([#12](https://github.com/tunix/valhalla/issues/12)) ([e4af1fa](https://github.com/tunix/valhalla/commit/e4af1fa26d15fb224e89ef00eaf9c1ef2e4b41aa))
* **ci:** skip systemd lifecycle test without user manager; make release publish idempotent ([#9](https://github.com/tunix/valhalla/issues/9)) ([142e0ce](https://github.com/tunix/valhalla/commit/142e0cef1a47305675418c9d7ea6d7ecefd59851))

## [0.2.0](https://github.com/tunix/valhalla/compare/v0.1.0...v0.2.0) (2026-09-27)


### Features

* adopt horizon logo as official app icon ([5e6b84b](https://github.com/tunix/valhalla/commit/5e6b84b3ee359bcb7d411cb3689699cd01c0730b))
* cached HTTP client for image downloads ([4da4201](https://github.com/tunix/valhalla/commit/4da420171b5d2f854ee62a5636fe85afc76aff3a))
* CLI entry point with tokio runtime and --daemon mode ([7d0bbf7](https://github.com/tunix/valhalla/commit/7d0bbf7baf78639521548761f866f7bfa4f1d27e))
* GSettings-backed config and GNOME Keyring credential storage ([26c419b](https://github.com/tunix/valhalla/commit/26c419bae73aa2c2dfd76f4352a99c71688a0851))
* libadwaita app shell with window and preferences ([bf6e6ec](https://github.com/tunix/valhalla/commit/bf6e6ecee5ecb8b44c6e28aaaba5b87f61eaf455))
* main window layout in Blueprint ([a3c6275](https://github.com/tunix/valhalla/commit/a3c6275d1e1c2ae064fe4aeed51eab1361cdc13e))
* perceptual brightness analysis for tone classification ([9333050](https://github.com/tunix/valhalla/commit/9333050cf19b76760c4e43cc037d45169f8c37fc))
* persisted state with cross-process refresh lock ([4a4b9c9](https://github.com/tunix/valhalla/commit/4a4b9c9a7706923d5c88245c618ffdf9dca3b6be))
* scheduler with theme matching, triggers and refresh timeout ([e56e9eb](https://github.com/tunix/valhalla/commit/e56e9ebfdfaa0a9cfd256820699e6220a4b2b437))
* screen probing and aspect-ratio mapping for wallpaper sources ([d94a8ee](https://github.com/tunix/valhalla/commit/d94a8eec29973a00ac4817f1b5d2d92ed09a8ccb))
* systemd user service management ([7673c9d](https://github.com/tunix/valhalla/commit/7673c9dfb795c6b456d87aec4c63c0b29c59e553))
* theme detection via XDG settings portal with gsettings fallback ([53b8589](https://github.com/tunix/valhalla/commit/53b85890d28dcebd43ccbd3a1fa8450ac4522b01))
* upscale wallpapers to cover the screen when smaller ([4272014](https://github.com/tunix/valhalla/commit/4272014f8ac48c68731c76e62d8c35968787289f))
* Wallhaven and Unsplash sources with tag, purity and orientation filters ([c6e6552](https://github.com/tunix/valhalla/commit/c6e6552ae2ca1dafe6bbf0e978a722679d8fcf5e))
* wallpaper application via portal and gsettings light/dark keys ([9bb2f9a](https://github.com/tunix/valhalla/commit/9bb2f9a2156149631036fc866da041a04397fe48))


### Bug Fixes

* **ci:** skip systemd lifecycle test without user manager; make release publish idempotent ([#9](https://github.com/tunix/valhalla/issues/9)) ([142e0ce](https://github.com/tunix/valhalla/commit/142e0cef1a47305675418c9d7ea6d7ecefd59851))

## 0.1.0 (2026-09-27)


### Features

* adopt horizon logo as official app icon ([5e6b84b](https://github.com/tunix/valhalla/commit/5e6b84b3ee359bcb7d411cb3689699cd01c0730b))
* cached HTTP client for image downloads ([4da4201](https://github.com/tunix/valhalla/commit/4da420171b5d2f854ee62a5636fe85afc76aff3a))
* CLI entry point with tokio runtime and --daemon mode ([7d0bbf7](https://github.com/tunix/valhalla/commit/7d0bbf7baf78639521548761f866f7bfa4f1d27e))
* GSettings-backed config and GNOME Keyring credential storage ([26c419b](https://github.com/tunix/valhalla/commit/26c419bae73aa2c2dfd76f4352a99c71688a0851))
* libadwaita app shell with window and preferences ([bf6e6ec](https://github.com/tunix/valhalla/commit/bf6e6ecee5ecb8b44c6e28aaaba5b87f61eaf455))
* main window layout in Blueprint ([a3c6275](https://github.com/tunix/valhalla/commit/a3c6275d1e1c2ae064fe4aeed51eab1361cdc13e))
* perceptual brightness analysis for tone classification ([9333050](https://github.com/tunix/valhalla/commit/9333050cf19b76760c4e43cc037d45169f8c37fc))
* persisted state with cross-process refresh lock ([4a4b9c9](https://github.com/tunix/valhalla/commit/4a4b9c9a7706923d5c88245c618ffdf9dca3b6be))
* scheduler with theme matching, triggers and refresh timeout ([e56e9eb](https://github.com/tunix/valhalla/commit/e56e9ebfdfaa0a9cfd256820699e6220a4b2b437))
* screen probing and aspect-ratio mapping for wallpaper sources ([d94a8ee](https://github.com/tunix/valhalla/commit/d94a8eec29973a00ac4817f1b5d2d92ed09a8ccb))
* systemd user service management ([7673c9d](https://github.com/tunix/valhalla/commit/7673c9dfb795c6b456d87aec4c63c0b29c59e553))
* theme detection via XDG settings portal with gsettings fallback ([53b8589](https://github.com/tunix/valhalla/commit/53b85890d28dcebd43ccbd3a1fa8450ac4522b01))
* upscale wallpapers to cover the screen when smaller ([4272014](https://github.com/tunix/valhalla/commit/4272014f8ac48c68731c76e62d8c35968787289f))
* Wallhaven and Unsplash sources with tag, purity and orientation filters ([c6e6552](https://github.com/tunix/valhalla/commit/c6e6552ae2ca1dafe6bbf0e978a722679d8fcf5e))
* wallpaper application via portal and gsettings light/dark keys ([9bb2f9a](https://github.com/tunix/valhalla/commit/9bb2f9a2156149631036fc866da041a04397fe48))
