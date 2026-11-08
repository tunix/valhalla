# Changelog

## [2.0.0](https://github.com/tunix/valhalla/compare/v1.6.2...v2.0.0) (2026-11-08)


### ⚠ BREAKING CHANGES

* drop systemd user service; portal-first everywhere ([#40](https://github.com/tunix/valhalla/issues/40))

### Features

* adopt horizon logo as official app icon ([5e6b84b](https://github.com/tunix/valhalla/commit/5e6b84b3ee359bcb7d411cb3689699cd01c0730b))
* cached HTTP client for image downloads ([4da4201](https://github.com/tunix/valhalla/commit/4da420171b5d2f854ee62a5636fe85afc76aff3a))
* CLI entry point with tokio runtime and --daemon mode ([7d0bbf7](https://github.com/tunix/valhalla/commit/7d0bbf7baf78639521548761f866f7bfa4f1d27e))
* **data:** flathub-ready appstream metadata ([#19](https://github.com/tunix/valhalla/issues/19)) ([c9bbd07](https://github.com/tunix/valhalla/commit/c9bbd078f80d572893d2297e335afe023ba972bf))
* GSettings-backed config and GNOME Keyring credential storage ([26c419b](https://github.com/tunix/valhalla/commit/26c419bae73aa2c2dfd76f4352a99c71688a0851))
* libadwaita app shell with window and preferences ([bf6e6ec](https://github.com/tunix/valhalla/commit/bf6e6ecee5ecb8b44c6e28aaaba5b87f61eaf455))
* main window layout in Blueprint ([a3c6275](https://github.com/tunix/valhalla/commit/a3c6275d1e1c2ae064fe4aeed51eab1361cdc13e))
* **manual:** folder-based sources with multi-folder looping ([#93](https://github.com/tunix/valhalla/issues/93)) ([9531b1c](https://github.com/tunix/valhalla/commit/9531b1c029d3911a24362e373608c32576c48a90))
* **packaging:** publish .flatpak bundles with each release ([#18](https://github.com/tunix/valhalla/issues/18)) ([0f0e346](https://github.com/tunix/valhalla/commit/0f0e3461f340b0b9c1b33b8f98754725dcee6fd4))
* perceptual brightness analysis for tone classification ([9333050](https://github.com/tunix/valhalla/commit/9333050cf19b76760c4e43cc037d45169f8c37fc))
* persisted state with cross-process refresh lock ([4a4b9c9](https://github.com/tunix/valhalla/commit/4a4b9c9a7706923d5c88245c618ffdf9dca3b6be))
* scheduler with theme matching, triggers and refresh timeout ([e56e9eb](https://github.com/tunix/valhalla/commit/e56e9ebfdfaa0a9cfd256820699e6220a4b2b437))
* screen probing and aspect-ratio mapping for wallpaper sources ([d94a8ee](https://github.com/tunix/valhalla/commit/d94a8eec29973a00ac4817f1b5d2d92ed09a8ccb))
* **source:** Pexels, Pixabay, Bing daily archive and NASA APOD sources ([e5f9173](https://github.com/tunix/valhalla/commit/e5f9173f7a6f81360e004cb23ad603aa319a1f76))
* systemd user service management ([7673c9d](https://github.com/tunix/valhalla/commit/7673c9dfb795c6b456d87aec4c63c0b29c59e553))
* theme detection via XDG settings portal with gsettings fallback ([53b8589](https://github.com/tunix/valhalla/commit/53b85890d28dcebd43ccbd3a1fa8450ac4522b01))
* **ui:** favorites management, instant refresh and window geometry persistence ([#79](https://github.com/tunix/valhalla/issues/79)) ([3f76294](https://github.com/tunix/valhalla/commit/3f762948872de162d17be6758ac0ba18e96d855d))
* **ui:** favorites source + heart toggle ([#49](https://github.com/tunix/valhalla/issues/49)) ([b7ef7d1](https://github.com/tunix/valhalla/commit/b7ef7d1b9c513f2bcc94859bfba471480239f995))
* **ui:** manual wallpaper source — pick files or a folder ([#53](https://github.com/tunix/valhalla/issues/53)) ([813a607](https://github.com/tunix/valhalla/commit/813a6077f92bf787de395ceca54954a3d945f8d7))
* **ui:** opt-in Start at login via XDG autostart ([#42](https://github.com/tunix/valhalla/issues/42)) ([05572bd](https://github.com/tunix/valhalla/commit/05572bdae1348f94b525cedc2770f1c89f22bcf0))
* **ui:** Preferences source sidebar with brand logos ([#77](https://github.com/tunix/valhalla/issues/77)) ([2e44962](https://github.com/tunix/valhalla/commit/2e4496223a208570acbfaeae235e737cfbbc7195))
* upscale wallpapers to cover the screen when smaller ([4272014](https://github.com/tunix/valhalla/commit/4272014f8ac48c68731c76e62d8c35968787289f))
* Wallhaven and Unsplash sources with tag, purity and orientation filters ([c6e6552](https://github.com/tunix/valhalla/commit/c6e6552ae2ca1dafe6bbf0e978a722679d8fcf5e))
* wallpaper application via portal and gsettings light/dark keys ([9bb2f9a](https://github.com/tunix/valhalla/commit/9bb2f9a2156149631036fc866da041a04397fe48))


### Bug Fixes

* **autostart:** Exec=valhalla --daemon via PATH, not current_exe ([#43](https://github.com/tunix/valhalla/issues/43)) ([80962dc](https://github.com/tunix/valhalla/commit/80962dc401b8a7e82e9ddf02c5c75f77a7e3dade))
* **autostart:** launch via flatpak run when running as a Flatpak ([4645b3a](https://github.com/tunix/valhalla/commit/4645b3ab69389b489bab6e30ce2533f4c5fc7a4f))
* **autostart:** launch via flatpak run when running as a Flatpak ([#90](https://github.com/tunix/valhalla/issues/90)) ([99ebbf6](https://github.com/tunix/valhalla/commit/99ebbf6f93c9c84a1d239300c17cbc3bb28d94e7))
* **autostart:** resolve Exec per packaging mode (AppImage via $APPIMAGE) ([#44](https://github.com/tunix/valhalla/issues/44)) ([0a75ece](https://github.com/tunix/valhalla/commit/0a75ece9d277af5b41ebdc9825ad236a6743b7e7))
* **ci:** accept update_metainfo.py's exit 1 in the sync step ([ed636e6](https://github.com/tunix/valhalla/commit/ed636e6166baf4a9d2060e8a31c975fa3c302ff8))
* **ci:** appimage sync step needs python3, installed before it runs ([b2406ed](https://github.com/tunix/valhalla/commit/b2406edec288e77f80dc75a620e7a9dc49c23ee9))
* **ci:** drop release-type input so release-please honors draft config ([9e1d965](https://github.com/tunix/valhalla/commit/9e1d96544c11b08193914818df8bd21d167dee80))
* **ci:** let release-please honor draft config (immutable-release failure) ([#11](https://github.com/tunix/valhalla/issues/11)) ([9e1d965](https://github.com/tunix/valhalla/commit/9e1d96544c11b08193914818df8bd21d167dee80))
* **ci:** move release-please config to repo root where the action looks ([#12](https://github.com/tunix/valhalla/issues/12)) ([4376d34](https://github.com/tunix/valhalla/commit/4376d3402914dc5d014ee3e75c721379122fec59))
* **ci:** release pipeline after systemd removal + AppImage VERSION scope ([#45](https://github.com/tunix/valhalla/issues/45)) ([8a5efa0](https://github.com/tunix/valhalla/commit/8a5efa0812c6f07ad10c80a3cbaea064ef9b0142))
* **ci:** run the flatpak job in the flatpak-github-actions container ([#23](https://github.com/tunix/valhalla/issues/23)) ([1d12fc5](https://github.com/tunix/valhalla/commit/1d12fc52ba99b356a207e1e710b16d3740ae9893))
* **ci:** skip systemd lifecycle test without user manager; make release publish idempotent ([#9](https://github.com/tunix/valhalla/issues/9)) ([388c24f](https://github.com/tunix/valhalla/commit/388c24fd2ac809d71701edfe6c7ecf2da2149354))
* **ci:** step-level TAG env on release uploads (dispatch path) ([#47](https://github.com/tunix/valhalla/issues/47)) ([0d562e2](https://github.com/tunix/valhalla/commit/0d562e263f7a3dab1071f4db98cc078c921cb84d))
* **ci:** sync appstream &lt;releases&gt; before bundling flatpak/AppImage ([87fde06](https://github.com/tunix/valhalla/commit/87fde066225a7db019203f2844695bfdbccb9a8f))
* **ci:** tag_name as direct expression in release upload steps ([#48](https://github.com/tunix/valhalla/issues/48)) ([faf99d2](https://github.com/tunix/valhalla/commit/faf99d2ddefaa5991a46a1fbe2290e0285d199aa))
* **deps:** update rust crate ashpd to 0.14.0 ([#100](https://github.com/tunix/valhalla/issues/100)) ([d3ba2b2](https://github.com/tunix/valhalla/commit/d3ba2b231e52ebefa9c2acd08ccb3d62fb9a6986))
* **manual:** folders only — drop single-file entries ([#98](https://github.com/tunix/valhalla/issues/98)) ([ef95430](https://github.com/tunix/valhalla/commit/ef954306421ec8c462245df72924ec297b856b53))
* **packaging:** ship gschemas.compiled next to the binary ([#97](https://github.com/tunix/valhalla/issues/97)) ([d32202d](https://github.com/tunix/valhalla/commit/d32202d9960037445a6406d9a1c82618bdba7d56))
* **release:** close the draft-mode duplicate-release race ([f10f561](https://github.com/tunix/valhalla/commit/f10f561341825fe4bf0e584a90e6e94f1007e56b))
* **release:** close the draft-mode duplicate-release race (force-tag-creation) ([#24](https://github.com/tunix/valhalla/issues/24)) ([f10f561](https://github.com/tunix/valhalla/commit/f10f561341825fe4bf0e584a90e6e94f1007e56b))
* **source:** append .jpg to bing /th URLs to stop 404s ([#103](https://github.com/tunix/valhalla/issues/103)) ([b9e8f04](https://github.com/tunix/valhalla/commit/b9e8f04ce56c21b680f4f983e392a67df2a33d2d))
* startup defaults on; systemd login-toggle fixes ([#38](https://github.com/tunix/valhalla/issues/38)) ([9749f8a](https://github.com/tunix/valhalla/commit/9749f8a0e2112c4901409af497a86f9dfe9b9a63))
* **ui:** About developer name; unfreeze the login-at-startup toggle ([#26](https://github.com/tunix/valhalla/issues/26)) ([73bf50b](https://github.com/tunix/valhalla/commit/73bf50b613725386282bba1a529e44e92c7de838))
* **ui:** embed the heart icon; theme lookup broke CSS recoloring ([#56](https://github.com/tunix/valhalla/issues/56)) ([cd52ad4](https://github.com/tunix/valhalla/commit/cd52ad44bf91e108a7a3b890f47e0c9c6b506c49))
* **ui:** heart button reflects favorite state; pop animation ([3535fb8](https://github.com/tunix/valhalla/commit/3535fb844a1438737a0f657f343fc74ed780367d))
* **ui:** mask API key rows as secrets with peek toggle ([#84](https://github.com/tunix/valhalla/issues/84)) ([fe63ca2](https://github.com/tunix/valhalla/commit/fe63ca2d281c7fa6f2c489cf46112acbeaeed829))
* **ui:** set minimum size on the preferences dialog ([#95](https://github.com/tunix/valhalla/issues/95)) ([1295737](https://github.com/tunix/valhalla/commit/1295737f9e81ccd98efcf5266c87fd45ed2cded4))
* **ui:** show the developer's name in About; freeze-proof the login toggle ([73bf50b](https://github.com/tunix/valhalla/commit/73bf50b613725386282bba1a529e44e92c7de838))
* **ui:** unlike animation + README screenshot refresh ([#61](https://github.com/tunix/valhalla/issues/61)) ([e71b4a7](https://github.com/tunix/valhalla/commit/e71b4a7121f15bf1d93a047b061b52f883e109eb))


### Performance Improvements

* optimize dependency crates in dev profile ([#94](https://github.com/tunix/valhalla/issues/94)) ([de34b7a](https://github.com/tunix/valhalla/commit/de34b7abfc7c6de88afa01b1b684f851af8b56d4))


### Code Refactoring

* drop systemd user service; portal-first everywhere ([#40](https://github.com/tunix/valhalla/issues/40)) ([8c7147d](https://github.com/tunix/valhalla/commit/8c7147d61bab658a4f890d10d4604786ba1430ca))

## [1.6.2](https://github.com/tunix/valhalla/compare/v1.6.1...v1.6.2) (2026-11-08)


### Bug Fixes

* **source:** append .jpg to bing /th URLs to stop 404s ([#103](https://github.com/tunix/valhalla/issues/103)) ([acecfe1](https://github.com/tunix/valhalla/commit/acecfe19cc5165ae62a9544f19a2201d7f11f807))

## [1.6.1](https://github.com/tunix/valhalla/compare/v1.6.0...v1.6.1) (2026-11-07)


### Bug Fixes

* **deps:** update rust crate ashpd to 0.14.0 ([#100](https://github.com/tunix/valhalla/issues/100)) ([ab7b536](https://github.com/tunix/valhalla/commit/ab7b536a6f8246bb3ca29c42a6520eb28551948b))
* **manual:** folders only — drop single-file entries ([#98](https://github.com/tunix/valhalla/issues/98)) ([d57ab33](https://github.com/tunix/valhalla/commit/d57ab332d4c82df0174348e2ec0835b338c9b535))

## [1.6.0](https://github.com/tunix/valhalla/compare/v1.5.2...v1.6.0) (2026-11-02)


### Features

* **manual:** folder-based sources with multi-folder looping ([#93](https://github.com/tunix/valhalla/issues/93)) ([30f2346](https://github.com/tunix/valhalla/commit/30f2346b0847136a6b503615330e2874d8540f8c))


### Bug Fixes

* **packaging:** ship gschemas.compiled next to the binary ([#97](https://github.com/tunix/valhalla/issues/97)) ([4ad904e](https://github.com/tunix/valhalla/commit/4ad904e89e53bde4030d0187b77bc3c0025967ae))
* **ui:** set minimum size on the preferences dialog ([#95](https://github.com/tunix/valhalla/issues/95)) ([bc33206](https://github.com/tunix/valhalla/commit/bc33206349c60718b64c53402c41d674f297c9da))


### Performance Improvements

* optimize dependency crates in dev profile ([#94](https://github.com/tunix/valhalla/issues/94)) ([e8766c9](https://github.com/tunix/valhalla/commit/e8766c96bf1cdbe239106ee2f24123af72cb7d49))

## [1.5.2](https://github.com/tunix/valhalla/compare/v1.5.1...v1.5.2) (2026-10-28)


### Bug Fixes

* **autostart:** launch via flatpak run when running as a Flatpak ([e62a73a](https://github.com/tunix/valhalla/commit/e62a73a085cb4f79e614f1a26d080b870ea69e88))
* **autostart:** launch via flatpak run when running as a Flatpak ([#90](https://github.com/tunix/valhalla/issues/90)) ([4bf8455](https://github.com/tunix/valhalla/commit/4bf8455e2429f031dd401b53c64cc8f7298c5f8d))

## [1.5.1](https://github.com/tunix/valhalla/compare/v1.5.0...v1.5.1) (2026-10-20)


### Bug Fixes

* **ui:** mask API key rows as secrets with peek toggle ([#84](https://github.com/tunix/valhalla/issues/84)) ([ebc206f](https://github.com/tunix/valhalla/commit/ebc206f802683b48153b0958a8c8cb969b9bd530))

## [1.5.0](https://github.com/tunix/valhalla/compare/v1.4.0...v1.5.0) (2026-10-20)


### Features

* **ui:** favorites management, instant refresh and window geometry persistence ([#79](https://github.com/tunix/valhalla/issues/79)) ([caf9eb9](https://github.com/tunix/valhalla/commit/caf9eb91585fed6cb125dad09f7d708622ca1474))

## [1.4.0](https://github.com/tunix/valhalla/compare/v1.3.4...v1.4.0) (2026-10-16)


### Features

* **ui:** Preferences source sidebar with brand logos ([#77](https://github.com/tunix/valhalla/issues/77)) ([bd6338d](https://github.com/tunix/valhalla/commit/bd6338d3769e26e56dbe735543cf0cad244c80c2))

## [1.3.4](https://github.com/tunix/valhalla/compare/v1.3.3...v1.3.4) (2026-10-16)


### Bug Fixes

* **ci:** appimage sync step needs python3, installed before it runs ([9ac34f5](https://github.com/tunix/valhalla/commit/9ac34f5c35ce8396c089c2f136891b33716e815d))

## [1.3.3](https://github.com/tunix/valhalla/compare/v1.3.2...v1.3.3) (2026-10-15)


### Bug Fixes

* **ci:** accept update_metainfo.py's exit 1 in the sync step ([73daa53](https://github.com/tunix/valhalla/commit/73daa537157181f3b081448a6cd11bd04e5cb8be))
* **ci:** sync appstream &lt;releases&gt; before bundling flatpak/AppImage ([07e663b](https://github.com/tunix/valhalla/commit/07e663b1e8bedef66a80add99275a058e0df3af9))

## [1.3.2](https://github.com/tunix/valhalla/compare/v1.3.1...v1.3.2) (2026-10-15)


### Bug Fixes

* **ui:** unlike animation + README screenshot refresh ([#61](https://github.com/tunix/valhalla/issues/61)) ([c2219ba](https://github.com/tunix/valhalla/commit/c2219bab5a3100bc1b143034486efe0a9e251d16))

## [1.3.1](https://github.com/tunix/valhalla/compare/v1.3.0...v1.3.1) (2026-10-13)


### Bug Fixes

* **ui:** embed the heart icon; theme lookup broke CSS recoloring ([#56](https://github.com/tunix/valhalla/issues/56)) ([90e1ce3](https://github.com/tunix/valhalla/commit/90e1ce32f711497944a0feebc49e48ff28a51160))

## [1.3.0](https://github.com/tunix/valhalla/compare/v1.2.0...v1.3.0) (2026-10-13)


### Features

* **source:** Pexels, Pixabay, Bing daily archive and NASA APOD sources ([ba5c37d](https://github.com/tunix/valhalla/commit/ba5c37de57cd407bfd617c7ec13388fb42fe05e7))


### Bug Fixes

* **ui:** heart button reflects favorite state; pop animation ([d15f8a2](https://github.com/tunix/valhalla/commit/d15f8a2df25d630df2edcf1f72e30bbe4e324e7a))

## [1.2.0](https://github.com/tunix/valhalla/compare/v1.1.0...v1.2.0) (2026-10-12)


### Features

* **ui:** manual wallpaper source — pick files or a folder ([#53](https://github.com/tunix/valhalla/issues/53)) ([7f051b3](https://github.com/tunix/valhalla/commit/7f051b3f8be36354191cc05893ef927d5497f99b))

## [1.1.0](https://github.com/tunix/valhalla/compare/v1.0.1...v1.1.0) (2026-10-12)


### Features

* **ui:** favorites source + heart toggle ([#49](https://github.com/tunix/valhalla/issues/49)) ([3972629](https://github.com/tunix/valhalla/commit/3972629f77a7ed7b5171b4b48a3c2f34cb93f2e2))

## [1.0.1](https://github.com/tunix/valhalla/compare/v1.0.0...v1.0.1) (2026-10-11)


### Bug Fixes

* **ci:** release pipeline after systemd removal + AppImage VERSION scope ([#45](https://github.com/tunix/valhalla/issues/45)) ([fa0c8e5](https://github.com/tunix/valhalla/commit/fa0c8e52784c359a6b155cadb39ecb26bc75cb1c))
* **ci:** step-level TAG env on release uploads (dispatch path) ([#47](https://github.com/tunix/valhalla/issues/47)) ([b6ddf61](https://github.com/tunix/valhalla/commit/b6ddf6123cf157a6b42b129bc6c1188dd658afae))
* **ci:** tag_name as direct expression in release upload steps ([#48](https://github.com/tunix/valhalla/issues/48)) ([7ce847d](https://github.com/tunix/valhalla/commit/7ce847da1cf739e2dd3bd4534b3b141782cab18a))

## [1.0.0](https://github.com/tunix/valhalla/compare/v0.5.1...v1.0.0) (2026-10-10)


### ⚠ BREAKING CHANGES

* drop systemd user service; portal-first everywhere ([#40](https://github.com/tunix/valhalla/issues/40))

### Features

* **ui:** opt-in Start at login via XDG autostart ([#42](https://github.com/tunix/valhalla/issues/42)) ([3b15358](https://github.com/tunix/valhalla/commit/3b1535887d359e9b3102a6df2ac2de69871fbadf))


### Bug Fixes

* **autostart:** Exec=valhalla --daemon via PATH, not current_exe ([#43](https://github.com/tunix/valhalla/issues/43)) ([5263adc](https://github.com/tunix/valhalla/commit/5263adcacd70a704f600ffa914b07f6de50f44f1))
* **autostart:** resolve Exec per packaging mode (AppImage via $APPIMAGE) ([#44](https://github.com/tunix/valhalla/issues/44)) ([4fa1f6e](https://github.com/tunix/valhalla/commit/4fa1f6e02afad99fc7a33c870fed77f576c548f9))
* startup defaults on; systemd login-toggle fixes ([#38](https://github.com/tunix/valhalla/issues/38)) ([9e42fcb](https://github.com/tunix/valhalla/commit/9e42fcbb97cc5442d47fbfcde7035484de778d76))


### Code Refactoring

* drop systemd user service; portal-first everywhere ([#40](https://github.com/tunix/valhalla/issues/40)) ([68a0251](https://github.com/tunix/valhalla/commit/68a0251399c8aac4701d41b073092f8fb288afab))

## [0.5.1](https://github.com/tunix/valhalla/compare/v0.5.0...v0.5.1) (2026-09-27)


### Bug Fixes

* **ui:** About developer name; unfreeze the login-at-startup toggle ([#26](https://github.com/tunix/valhalla/issues/26)) ([6bcca47](https://github.com/tunix/valhalla/commit/6bcca478aed338cd576640943daad109118eb704))
* **ui:** show the developer's name in About; freeze-proof the login toggle ([6bcca47](https://github.com/tunix/valhalla/commit/6bcca478aed338cd576640943daad109118eb704))

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
