#!/usr/bin/env bash

AIRBREAK_DEFAULT_UI_SCREENS="block_breaker,custom_about,clinical_mode"
AIRBREAK_MY_OPTIONS_AIRBREAK_SECTION_STEPS="${AIRBREAK_MY_OPTIONS_AIRBREAK_SECTION_STEPS:-5}"
AIRBREAK_FRONT_PANEL_ENTER_MY_OPTIONS_AT="${AIRBREAK_FRONT_PANEL_ENTER_MY_OPTIONS_AT:-500000000}"
AIRBREAK_FRONT_PANEL_FIRST_ROTATE_AT="${AIRBREAK_FRONT_PANEL_FIRST_ROTATE_AT:-570000000}"
AIRBREAK_FRONT_PANEL_ROTATE_INTERVAL="${AIRBREAK_FRONT_PANEL_ROTATE_INTERVAL:-40000000}"
AIRBREAK_FRONT_PANEL_SELECT_AFTER="${AIRBREAK_FRONT_PANEL_SELECT_AFTER:-20000000}"
AIRBREAK_CLINICAL_SETTINGS_ENTER_DELAY="${AIRBREAK_CLINICAL_SETTINGS_ENTER_DELAY:-100000000}"
AIRBREAK_CLINICAL_SETTINGS_ABOUT_STEPS="${AIRBREAK_CLINICAL_SETTINGS_ABOUT_STEPS:-30}"
AIRBREAK_CLINICAL_SETTINGS_ABOUT_FIRST_ROTATE_DELAY="${AIRBREAK_CLINICAL_SETTINGS_ABOUT_FIRST_ROTATE_DELAY:-160000000}"
AIRBREAK_CLINICAL_SETTINGS_ABOUT_ROTATE_INTERVAL="${AIRBREAK_CLINICAL_SETTINGS_ABOUT_ROTATE_INTERVAL:-5000000}"
AIRBREAK_CLINICAL_SETTINGS_ABOUT_ENTER_DELAY="${AIRBREAK_CLINICAL_SETTINGS_ABOUT_ENTER_DELAY:-330000000}"
AIRBREAK_CLINICAL_SETTINGS_ABOUT_BACK_DELAY="${AIRBREAK_CLINICAL_SETTINGS_ABOUT_BACK_DELAY:-430000000}"

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

airbreak_ui_front_panel_select_at_for_target() {
  local target="$1"
  local index
  local steps

  index="$(airbreak_ui_target_index "${target}")" || return $?
  steps=$((AIRBREAK_MY_OPTIONS_AIRBREAK_SECTION_STEPS + index))
  printf '%s\n' "$((AIRBREAK_FRONT_PANEL_FIRST_ROTATE_AT + (steps * AIRBREAK_FRONT_PANEL_ROTATE_INTERVAL) + AIRBREAK_FRONT_PANEL_SELECT_AFTER))"
}

airbreak_ui_sequence_flow_known() {
  case "$1" in
    ""|"-"|clinical_settings_about_return) return 0 ;;
    *)
      echo "airbreak_ui=fail reason=unknown_sequence_flow flow=$1 result=fail" >&2
      return 1
      ;;
  esac
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

airbreak_ui_front_panel_sequence_for_case() {
  local target="$1"
  local flow="${2:-}"
  local sequence
  local select_at
  local rotate_at
  local i

  airbreak_ui_sequence_flow_known "${flow}" || return $?
  sequence="$(airbreak_ui_front_panel_sequence_for_target "${target}")" || return $?
  case "${flow}" in
    ""|"-")
      printf '%s\n' "${sequence}"
      ;;
    clinical_settings_about_return)
      if [[ "${target}" != "clinical_mode" ]]; then
        echo "airbreak_ui=fail reason=flow_target_mismatch flow=${flow} target=${target} result=fail" >&2
        return 1
      fi
      select_at="$(airbreak_ui_front_panel_select_at_for_target "${target}")" || return $?
      sequence="${sequence},encoder@$((select_at + AIRBREAK_CLINICAL_SETTINGS_ENTER_DELAY))"
      i=0
      while (( i < AIRBREAK_CLINICAL_SETTINGS_ABOUT_STEPS )); do
        rotate_at=$((select_at + AIRBREAK_CLINICAL_SETTINGS_ABOUT_FIRST_ROTATE_DELAY + (i * AIRBREAK_CLINICAL_SETTINGS_ABOUT_ROTATE_INTERVAL)))
        sequence="${sequence},cw@${rotate_at}"
        i=$((i + 1))
      done
      sequence="${sequence},encoder@$((select_at + AIRBREAK_CLINICAL_SETTINGS_ABOUT_ENTER_DELAY))"
      sequence="${sequence},encoder@$((select_at + AIRBREAK_CLINICAL_SETTINGS_ABOUT_BACK_DELAY))"
      printf '%s\n' "${sequence}"
      ;;
  esac
}
