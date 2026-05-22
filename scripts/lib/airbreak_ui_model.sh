#!/usr/bin/env bash

AIRBREAK_DEFAULT_UI_SCREENS="block_breaker,custom_about,clinical_mode"
AIRBREAK_MY_OPTIONS_AIRBREAK_SECTION_STEPS="${AIRBREAK_MY_OPTIONS_AIRBREAK_SECTION_STEPS:-5}"
AIRBREAK_FRONT_PANEL_ENTER_MY_OPTIONS_AT="${AIRBREAK_FRONT_PANEL_ENTER_MY_OPTIONS_AT:-500000000}"
AIRBREAK_FRONT_PANEL_FIRST_ROTATE_AT="${AIRBREAK_FRONT_PANEL_FIRST_ROTATE_AT:-570000000}"
AIRBREAK_FRONT_PANEL_ROTATE_INTERVAL="${AIRBREAK_FRONT_PANEL_ROTATE_INTERVAL:-40000000}"
AIRBREAK_FRONT_PANEL_SELECT_AFTER="${AIRBREAK_FRONT_PANEL_SELECT_AFTER:-20000000}"

airbreak_ui_screen_id() {
  case "$1" in
    "") echo 0 ;;
    block_breaker) echo 1 ;;
    custom_about) echo 2 ;;
    clinical_mode) echo 3 ;;
    *)
      echo "airbreak_ui=fail reason=unknown_ui_screen screen=$1 result=fail" >&2
      return 1
      ;;
  esac
}

airbreak_ui_default_screens() {
  if [[ -n "${AIRBREAK_ENABLE_BLOCK_BREAKER+x}" && -z "${AIRBREAK_UI_SCREENS+x}" ]]; then
    if [[ "${AIRBREAK_ENABLE_BLOCK_BREAKER}" == "0" ]]; then
      printf '%s\n' "custom_about,clinical_mode"
      return
    fi
  fi
  printf '%s\n' "${AIRBREAK_UI_SCREENS:-${AIRBREAK_DEFAULT_UI_SCREENS}}"
}

airbreak_ui_screen_enabled() {
  case ",${AIRBREAK_UI_SCREENS}," in
    *",$1,"*) return 0 ;;
    *) return 1 ;;
  esac
}

airbreak_ui_configure() {
  local screens="${1:-$(airbreak_ui_default_screens)}"
  local screen
  local seen=","
  local screen_id
  local model_init=""

  AIRBREAK_UI_SCREENS="${screens//[[:space:]]/}"
  IFS=',' read -r -a AIRBREAK_UI_SCREEN_LIST <<< "${AIRBREAK_UI_SCREENS}"

  if [[ -z "${AIRBREAK_UI_SCREENS//,/}" ]]; then
    echo "airbreak_ui=fail reason=empty_ui_screens result=fail" >&2
    return 1
  fi

  for screen in "${AIRBREAK_UI_SCREEN_LIST[@]}"; do
    if [[ -z "${screen}" ]]; then
      echo "airbreak_ui=fail reason=empty_ui_screen_entry screens=${AIRBREAK_UI_SCREENS} result=fail" >&2
      return 1
    fi
    screen_id="$(airbreak_ui_screen_id "${screen}")" || return $?
    case "${seen}" in
      *",${screen},"*)
        echo "airbreak_ui=fail reason=duplicate_ui_screen screen=${screen} result=fail" >&2
        return 1
        ;;
    esac
    seen="${seen}${screen},"
    if [[ -n "${model_init}" ]]; then
      model_init="${model_init},"
    fi
    model_init="${model_init}${screen_id}u"
  done

  AIRBREAK_UI_SCREEN_MODEL_COUNT="${#AIRBREAK_UI_SCREEN_LIST[@]}u"
  AIRBREAK_UI_SCREEN_MODEL_INIT="${model_init}"

  AIRBREAK_UI_HAS_BLOCK_BREAKER=0
  AIRBREAK_UI_HAS_CUSTOM_ABOUT=0
  AIRBREAK_UI_HAS_CLINICAL_MODE=0
  AIRBREAK_UI_ROW_COUNT="${#AIRBREAK_UI_SCREEN_LIST[@]}"
  if airbreak_ui_screen_enabled block_breaker; then
    AIRBREAK_UI_HAS_BLOCK_BREAKER=1
  fi
  if airbreak_ui_screen_enabled custom_about; then
    AIRBREAK_UI_HAS_CUSTOM_ABOUT=1
  fi
  if airbreak_ui_screen_enabled clinical_mode; then
    AIRBREAK_UI_HAS_CLINICAL_MODE=1
  fi
}

airbreak_ui_target_index() {
  local target="$1"
  local i
  for i in "${!AIRBREAK_UI_SCREEN_LIST[@]}"; do
    if [[ "${AIRBREAK_UI_SCREEN_LIST[$i]}" == "${target}" ]]; then
      printf '%s\n' "${i}"
      return 0
    fi
  done
  return 1
}

airbreak_ui_front_panel_sequence_for_target() {
  local target="$1"
  local index
  local steps
  local rotate_at
  local select_at
  local i
  local sequence

  index="$(airbreak_ui_target_index "${target}")" || return $?
  steps=$((AIRBREAK_MY_OPTIONS_AIRBREAK_SECTION_STEPS + index))
  sequence="encoder@${AIRBREAK_FRONT_PANEL_ENTER_MY_OPTIONS_AT}"

  i=0
  while (( i < steps )); do
    rotate_at=$((AIRBREAK_FRONT_PANEL_FIRST_ROTATE_AT + (i * AIRBREAK_FRONT_PANEL_ROTATE_INTERVAL)))
    sequence="${sequence},cw@${rotate_at}"
    i=$((i + 1))
  done

  select_at=$((AIRBREAK_FRONT_PANEL_FIRST_ROTATE_AT + (steps * AIRBREAK_FRONT_PANEL_ROTATE_INTERVAL) + AIRBREAK_FRONT_PANEL_SELECT_AFTER))
  sequence="${sequence},encoder@${select_at}"
  printf '%s\n' "${sequence}"
}
