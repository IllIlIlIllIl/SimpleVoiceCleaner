# SimpleVoiceCleaner v11 / SimpleVoiceCleaner rename

Rust/nih-plug 기반 실시간 마이크용 VST3/CLAP 플러그인입니다.

## 변경사항

- Denoise Amount를 `Denoise Reduction dB`로 변경
- 기본값을 `100 dB`로 변경
- GUI에서 바꾼 모든 설정을 자동 저장
- 다음 실행/로드 시 저장된 설정 자동 로드

## 처리 순서

```text
Input
→ HPF 75Hz
→ Light Denoiser / adaptive expander
→ Vocal Rider
→ Output Gain
→ Safety Limiter
→ Output
```

## 자동 저장 위치

Windows 기준:

```text
%APPDATA%\SimpleVoiceCleaner\settings.txt
```

설정을 완전히 초기화하고 싶으면 위 파일을 삭제하면 됩니다. 그러면 다음 로드 시 기본값으로 돌아갑니다.

## 중요

현재 Denoiser는 DeepFilterNet 같은 AI 디노이저가 아니라, 실시간용 adaptive expander 방식입니다. `100 dB`는 Alt Denoiser의 AI suppression과 같은 의미가 아니라, 이 플러그인 내부에서 잡음 구간에 적용할 수 있는 최대 감쇠량입니다.

## 빌드

Windows에서 압축을 풀고:

```bat
build_vst3.bat
```

결과물:

```text
target\bundled\simple_voice_cleaner.vst3
target\bundled\simple_voice_cleaner.clap
```

## 설치

VST3 폴더로 복사:

```text
C:\Program Files\Common Files\VST3
```

또는 관리자 권한으로:

```bat
build_and_install_vst3_admin.bat
```

## 기본값

```text
Denoiser: On
HPF 75Hz: On
Denoise Reduction: 100 dB
Denoise Floor: -55 dB
Denoise Softness: 12 dB
Target: -18 dB
Ride Amount: 70%
Speed: 500 ms
Rider Floor: -50 dB
Max Boost: +6 dB
Max Cut: -9 dB
Output Gain: 0 dB
Safety Limiter: On
```
