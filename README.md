# KakaoTalk AdBlock

<p align="center">
  <img src="https://img.shields.io/github/v/release/ssut/kakaotalk-adblock-rs?style=flat-square" alt="Release">
  <img src="https://img.shields.io/github/downloads/ssut/kakaotalk-adblock-rs/total?style=flat-square" alt="Downloads">
  <img src="https://img.shields.io/github/license/ssut/kakaotalk-adblock-rs?style=flat-square" alt="License">
</p>

카카오톡 PC 버전의 광고를 제거하는 프로그램입니다.

A program that removes ads from KakaoTalk PC client.

---

## 한국어

### 기능

- 메인 화면 하단 광고 배너 제거
- 잠금 화면 광고 제거  
- 팝업 광고 창 차단
- 시스템 트레이에서 실행 (백그라운드)
- 시작 시 자동 실행 설정
- 새 버전 알림

### 다운로드

[Releases](https://github.com/ssut/kakaotalk-adblock-rs/releases) 페이지에서 최신 버전을 다운로드하세요.

### 사용법

1. `KakaoTalkAdBlock-YYYYMMDD-NN.exe` 다운로드
2. 실행 (시스템 트레이에 아이콘 표시)
3. 카카오톡 실행 - 광고가 자동으로 제거됩니다

### 트레이 메뉴

시스템 트레이 아이콘을 우클릭하면:

| 메뉴 | 설명 |
|------|------|
| 버전 | 현재 버전 표시 |
| 업데이트 확인 | 새 버전이 있으면 표시, 클릭 시 릴리즈 페이지 이동 |
| 디버그 창 표시 | 디버그 정보 오버레이 창 토글 |
| 시작 시 자동 실행 | Windows 시작 시 자동 실행 설정 |
| 종료 | 프로그램 종료 |

### 직접 빌드

```bash
# Rust 설치 필요 (https://rustup.rs)
git clone https://github.com/ssut/kakaotalk-adblock-rs.git
cd kakaotalk-adblock-rs
cargo build --release

# 결과: target/release/kakaotalk_adblock.exe
```

### 작동 원리

1. KakaoTalk 프로세스의 윈도우 핸들을 모니터링
2. 광고 관련 윈도우 클래스(`EVA_Window`, `EVA_ChildWindow` 등) 탐지
3. 광고 영역 리사이즈 또는 숨김 처리
4. Chrome Legacy Window (광고 팝업) 차단

### 문제 해결

**광고가 제거되지 않아요**
- 카카오톡을 재시작해보세요
- KakaoTalkAdBlock이 실행 중인지 확인하세요 (시스템 트레이)

**프로그램이 실행되지 않아요**
- 이미 실행 중인지 확인하세요 (중복 실행 방지됨)
- Windows Defender/백신에서 차단되지 않았는지 확인하세요

---

## English

### Features

- Removes bottom ad banner from main window
- Removes lock screen ads
- Blocks popup ad windows
- Runs in system tray (background)
- Auto-start with Windows option
- New version notifications

### Download

Download the latest version from [Releases](https://github.com/ssut/kakaotalk-adblock-rs/releases).

### Usage

1. Download `KakaoTalkAdBlock-YYYYMMDD-NN.exe`
2. Run it (icon appears in system tray)
3. Launch KakaoTalk - ads will be automatically removed

### Tray Menu

Right-click the system tray icon:

| Menu | Description |
|------|-------------|
| Version | Shows current version |
| Check for updates | Shows if new version available, click to open releases page |
| Show debug window | Toggle debug info overlay |
| Run on startup | Enable/disable auto-start with Windows |
| Exit | Close the program |

### Building from Source

```bash
# Requires Rust (https://rustup.rs)
git clone https://github.com/ssut/kakaotalk-adblock-rs.git
cd kakaotalk-adblock-rs
cargo build --release

# Output: target/release/kakaotalk_adblock.exe
```

### How It Works

1. Monitors window handles of KakaoTalk process
2. Detects ad-related window classes (`EVA_Window`, `EVA_ChildWindow`, etc.)
3. Resizes or hides ad areas
4. Blocks Chrome Legacy Window (ad popups)

### Troubleshooting

**Ads are not being removed**
- Try restarting KakaoTalk
- Check if KakaoTalkAdBlock is running (system tray)

**Program won't start**
- Check if it's already running (duplicate instances are prevented)
- Check if Windows Defender/antivirus is blocking it

---

## License

MIT License

## Credits

- Original concept: [blurfx/KakaoTalkAdBlock](https://github.com/blurfx/KakaoTalkAdBlock)
- Rust rewrite: [ssut](https://github.com/ssut)
