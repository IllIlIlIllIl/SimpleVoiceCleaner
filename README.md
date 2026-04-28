# SimpleVoiceCleaner

SimpleVoiceCleaner는 실시간 마이크 음성을 위한 VST3 플러그인입니다.

LightHost에서 VST3 플러그인으로 실행한 뒤, VB-CABLE을 통해 OBS, Discord, 게임, 통화 프로그램 등으로 처리된 마이크 소리를 전달하는 사용 방식을 권장합니다.


## 주요 기능

- 실시간 마이크 입력 처리
- Light Denoiser / Adaptive Expander 방식의 간단한 잡음 감소
- Vocal Rider / Voice Leveler 방식의 음량 보정
- HPF 75Hz
- Output Gain
- Safety Limiter
- GUI 기반 설정 조정
- 설정 자동 저장 및 자동 로드


## 권장 사용 방식

SimpleVoiceCleaner는 OBS에 직접 VST2 플러그인으로 넣는 방식이 아니라, LightHost에서 VST3 플러그인으로 실행한 뒤 VB-CABLE을 통해 OBS나 다른 프로그램으로 전달하는 방식을 권장합니다.

```
실제 마이크
→ LightHost
→ SimpleVoiceCleaner
→ CABLE Input
→ CABLE Output
→ OBS / Discord / 게임 / 기타 프로그램
