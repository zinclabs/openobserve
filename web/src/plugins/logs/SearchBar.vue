<!-- Copyright 2023 OpenObserve Inc.

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
-->

<template>
  <div class="logs-search-bar-component" id="searchBarComponent">
    <div class="row">
      <div class="float-right col q-mb-xs flex">
        <div class="button-group logs-visualize-toggle q-ml-xs">
          <div class="row">
            <div>
              <button
                data-test="logs-logs-toggle"
                :class="
                  searchObj.meta.logsVisualizeToggle === 'logs'
                    ? 'selected'
                    : ''
                "
                class="button button-left"
                @click="onLogsVisualizeToggleUpdate('logs')"
              >
                Search
              </button>
            </div>
            <div>
              <button
                data-test="logs-visualize-toggle"
                :class="
                  searchObj.meta.logsVisualizeToggle === 'visualize'
                    ? 'selected'
                    : ''
                "
                class="button button-right"
                @click="onLogsVisualizeToggleUpdate('visualize')"
                :disabled="isVisualizeToggleDisabled"
                :title="[
                  isVisualizeToggleDisabled
                    ? 'Visualization is disabled for multi stream'
                    : '',
                ]"
              >
                Visualize
              </button>
            </div>
          </div>
        </div>
        <q-toggle
          data-test="logs-search-bar-show-histogram-toggle-btn"
          v-model="searchObj.meta.showHistogram"
          :label="t('search.showHistogramLabel')"
        />
        <q-toggle
          data-test="logs-search-bar-sql-mode-toggle-btn"
          v-model="searchObj.meta.sqlMode"
          :label="t('search.sqlModeLabel')"
        />
        <q-btn
          data-test="logs-search-bar-reset-filters-btn"
          :title="t('search.resetFilters')"
          no-caps
          size="sm"
          icon="restart_alt"
          class="tw-flex tw-justify-center tw-items-center reset-filters q-ml-xs"
          @click="resetFilters"
        />
        <syntax-guide
          data-test="logs-search-bar-sql-mode-toggle-btn"
          :sqlmode="searchObj.meta.sqlMode"
        ></syntax-guide>
        <q-btn-group class="q-ml-xs no-outline q-pa-none no-border">
          <q-btn-dropdown
            data-test="logs-search-saved-views-btn"
            v-model="savedViewDropdownModel"
            size="12px"
            icon="save"
            icon-right="saved_search"
            :title="t('search.savedViewsLabel')"
            @click="fnSavedView"
            @show="loadSavedView"
            split
            class="no-outline saved-views-dropdown no-border"
          >
            <q-list
              :style="
                localSavedViews.length > 0 ? 'width: 500px' : 'width: 250px'
              "
              data-test="logs-search-saved-view-list"
            >
              <q-item style="padding: 0px 0px 0px 0px">
                <q-item-section
                  class="column"
                  no-hover
                  style="width: 60%; border-right: 1px solid lightgray"
                >
                  <q-table
                    data-test="log-search-saved-view-list-fields-table"
                    :visible-columns="['view_name']"
                    :rows="searchObj.data.savedViews"
                    :row-key="(row) => 'saved_view_' + row.view_id"
                    :filter="searchObj.data.savedViewFilterFields"
                    :filter-method="filterSavedViewFn"
                    :pagination="{ rowsPerPage }"
                    hide-header
                    :wrap-cells="searchObj.meta.resultGrid.wrapCells"
                    class="saved-view-table full-height"
                    no-hover
                    id="savedViewList"
                    :rows-per-page-options="[]"
                    :hide-bottom="
                      searchObj.data.savedViews.length <= rowsPerPage ||
                      searchObj.data.savedViews.length == 0
                    "
                  >
                    <template #top-right>
                      <div class="full-width">
                        <q-input
                          data-test="log-search-saved-view-field-search-input"
                          v-model="searchObj.data.savedViewFilterFields"
                          data-cy="index-field-search-input"
                          filled
                          borderless
                          dense
                          clearable
                          debounce="1"
                          :placeholder="t('search.searchSavedView')"
                        >
                          <template #prepend>
                            <q-icon name="search" />
                          </template>
                        </q-input>
                      </div>
                      <div
                        v-if="searchObj.loadingSavedView == true"
                        class="full-width float-left"
                      >
                        <div class="text-subtitle2 text-weight-bold float-left">
                          <q-spinner-hourglass size="20px" />
                          {{ t("confirmDialog.loading") }}
                        </div>
                      </div>
                      <q-tr>
                        <q-td
                          v-if="
                            searchObj.data.savedViews.length == 0 &&
                            searchObj.loadingSavedView == false
                          "
                        >
                          <q-item-label class="q-pl-sm q-pt-sm">{{
                            t("search.savedViewsNotFound")
                          }}</q-item-label>
                        </q-td>
                      </q-tr>
                    </template>
                    <template v-slot:body-cell-view_name="props">
                      <q-td :props="props" class="field_list" no-hover>
                        <q-item
                          class="q-pa-sm saved-view-item"
                          clickable
                          v-close-popup
                        >
                          <q-item-section
                            @click.stop="applySavedView(props.row)"
                            v-close-popup
                            :title="props.row.view_name"
                          >
                            <q-item-label
                              class="ellipsis"
                              style="max-width: 188px"
                              >{{ props.row.view_name }}</q-item-label
                            >
                          </q-item-section>
                          <q-item-section
                            :data-test="`logs-search-bar-favorite-${props.row.view_name}-saved-view-btn`"
                            side
                            @click.stop="
                              handleFavoriteSavedView(
                                props.row,
                                favoriteViews.includes(props.row.view_id),
                              )
                            "
                          >
                            <q-btn
                              :icon="favoriteViews.includes(props.row.view_id)
                                  ? 'favorite'
                                  : 'favorite_border'"
                              :title="t('common.favourite')"
                              class="logs-saved-view-icon"
                              padding="xs"
                              unelevated
                              size="xs"
                              round
                              flat
                            ></q-btn>
                          </q-item-section>
                          <q-item-section
                            :data-test="`logs-search-bar-update-${props.row.view_name}-saved-view-btn`"
                            side
                            @click.stop="handleUpdateSavedView(props.row)"
                          >
                            <q-btn
                              icon="edit"
                              :title="t('common.edit')"
                              class="logs-saved-view-icon"
                              padding="xs"
                              unelevated
                              size="xs"
                              round
                              flat
                            ></q-btn>
                          </q-item-section>
                          <q-item-section
                            :data-test="`logs-search-bar-delete-${props.row.view_name}-saved-view-btn`"
                            side
                            @click.stop="handleDeleteSavedView(props.row)"
                          >
                            <q-btn
                              icon="delete"
                              :title="t('common.delete')"
                              class="logs-saved-view-icon"
                              padding="xs"
                              unelevated
                              size="xs"
                              round
                              flat
                            ></q-btn>
                          </q-item-section>
                        </q-item> </q-td
                    ></template>
                  </q-table>
                </q-item-section>

                <q-item-section
                  class="column"
                  style="width: 40%; margin-left: 0px"
                  v-if="localSavedViews.length > 0"
                >
                  <q-table
                    data-test="log-search-saved-view-favorite-list-fields-table"
                    :visible-columns="['view_name']"
                    :rows="localSavedViews"
                    :row-key="(row) => 'favorite_saved_view_' + row.view_name"
                    hide-header
                    hide-bottom
                    :wrap-cells="searchObj.meta.resultGrid.wrapCells"
                    class="saved-view-table full-height"
                    id="savedViewFavoriteList"
                    :rows-per-page-options="[0]"
                  >
                    <template #top-right>
                      <q-item style="padding: 0px"
                        ><q-item-label
                          header
                          class="q-pa-sm text-bold favorite-label"
                          >Favorite Views</q-item-label
                        ></q-item
                      >
                      <q-separator horizontal inset></q-separator>
                    </template>
                    <template v-slot:body-cell-view_name="props">
                      <q-td :props="props" class="field_list q-pa-xs">
                        <q-item
                          class="q-pa-sm saved-view-item"
                          clickable
                          v-close-popup
                        >
                          <q-item-section
                            @click.stop="applySavedView(props.row)"
                            v-close-popup
                          >
                            <q-item-label
                              class="ellipsis"
                              style="max-width: 185px"
                              >{{ props.row.view_name }}</q-item-label
                            >
                          </q-item-section>
                          <q-item-section
                            :data-test="`logs-search-bar-favorite-${props.row.view_name}-saved-view-btn`"
                            side
                            @click.stop="
                              handleFavoriteSavedView(
                                props.row,
                                favoriteViews.includes(props.row.view_id),
                              )
                            "
                          >
                            <q-icon
                              :name="
                                favoriteViews.includes(props.row.view_id)
                                  ? 'favorite'
                                  : 'favorite_border'
                              "
                              color="grey"
                              size="xs"
                            />
                          </q-item-section>
                        </q-item> </q-td
                    ></template>
                  </q-table>
                </q-item-section>
              </q-item>
            </q-list>
          </q-btn-dropdown>
        </q-btn-group>
        <q-toggle
          data-test="logs-search-bar-quick-mode-toggle-btn"
          v-model="searchObj.meta.quickMode"
          :label="t('search.quickModeLabel')"
          @click="handleQuickMode"
        />
      </div>
      <div class="float-right col-auto q-mb-xs">
        <q-toggle
          data-test="logs-search-bar-wrap-table-content-toggle-btn"
          v-model="searchObj.meta.toggleSourceWrap"
          icon="wrap_text"
          :title="t('search.messageWrapContent')"
          class="float-left"
          size="32px"
        />
        <q-toggle
          data-test="logs-search-bar-show-query-toggle-btn"
          v-model="searchObj.meta.toggleFunction"
          :icon="functionToggleIcon"
          title="Toggle Function Editor"
          class="float-left"
          size="32px"
        />
        <q-btn-group
          class="no-outline q-pa-none no-border float-left q-mr-xs"
          :disable="!searchObj.meta.toggleFunction"
        >
          <q-btn-dropdown
            data-test="logs-search-bar-function-dropdown"
            v-model="functionModel"
            size="12px"
            icon="save"
            :icon-right="iconRight"
            :title="t('search.functionPlaceholder')"
            split
            class="no-outline saved-views-dropdown no-border btn-function"
            @click="fnSavedFunctionDialog"
          >
            <q-list data-test="logs-search-saved-function-list">
              <!-- Search Input -->
              <div>
                <q-input
                  v-model="searchTerm"
                  dense
                  filled
                  borderless
                  clearable
                  debounce="300"
                  :placeholder="t('search.searchSavedFunction')"
                  data-test="function-search-input"
                >
                  <template #prepend>
                    <q-icon name="search" />
                  </template>
                </q-input>
              </div>

              <div v-if="filteredFunctionOptions.length">
                <q-item
                  class="tw-border-b saved-view-item"
                  clickable
                  v-for="(item, i) in filteredFunctionOptions"
                  :key="'saved-view-' + i"
                  v-close-popup
                >
                  <q-item-section
                    @click.stop="populateFunctionImplementation(item, true)"
                    v-close-popup
                  >
                    <q-item-label>{{ item.name }}</q-item-label>
                  </q-item-section>
                </q-item>
              </div>
              <div v-else>
                <q-item>
                  <q-item-section>
                    <q-item-label>{{
                      t("search.savedFunctionNotFound")
                    }}</q-item-label>
                  </q-item-section>
                </q-item>
              </div>
            </q-list>
          </q-btn-dropdown>
        </q-btn-group>
        <q-btn-group v-if="config.isEnterprise == 'true'" class=" no-outline q-pa-none no-border">
          <q-btn-dropdown
            data-test="search-scheduler-dropdown-btn-group"
            class="q-mr-xs download-logs-btn q-px-xs "
            size="sm"
            icon="schedule_send"
            :title="t('search.exportLogs')"
          >
            <q-list>
              <q-item
                data-test="search-scheduler-create-new-btn"
                class="q-pa-sm saved-view-item"
                clickable
                v-close-popup
                @click="createScheduleJob"
              >
                <q-item-section v-close-popup>
                  <q-item-label data-test="search-scheduler-create-new-label"
                  >
                    <q-icon name="save" />
                    Create Scheduled Search</q-item-label
                  >
                </q-item-section>
              </q-item>
              <q-separator />
              <q-item
                data-test="search-scheduler-list-btn"

                class="q-pa-sm saved-view-item"
                clickable
                v-close-popup
                @click="routeToSearchSchedule"
              >
                <q-item-section v-close-popup>
                  <q-item-label data-test="search-scheduler-list-label"
                  >
                    <q-icon name="list" />

                    List Scheduled Search</q-item-label
                  >
                </q-item-section>
              </q-item>
            </q-list>
          </q-btn-dropdown>
        </q-btn-group>
        <q-btn-group class="no-outline q-pa-none no-border">
          <q-btn-dropdown
            data-test="logs-search-bar-reset-function-btn"
            class="q-mr-xs download-logs-btn q-px-xs"
            size="sm"
            icon="download"
            :title="t('search.exportLogs')"
          >
            <q-list>
              <q-item
                class="q-pa-sm saved-view-item"
                clickable
                v-close-popup
                v-bind:disable="
                  searchObj.data.queryResults &&
                  searchObj.data.queryResults.hasOwnProperty('hits') &&
                  !searchObj.data.queryResults.hits.length
                "
              >
                <q-item-section
                  @click.stop="downloadLogs(searchObj.data.queryResults.hits)"
                  v-close-popup
                >
                  <q-item-label>{{ t("search.downloadTable") }}</q-item-label>
                </q-item-section>
              </q-item>
              <q-separator />
              <q-item class="q-pa-sm saved-view-item" clickable v-close-popup>
                <q-item-section
                  @click.stop="toggleCustomDownloadDialog"
                  v-close-popup
                >
                  <q-item-label>{{ t("search.customRange") }}</q-item-label>
                </q-item-section>
              </q-item>
            </q-list>
          </q-btn-dropdown>
        </q-btn-group>
        <q-btn
          data-test="logs-search-bar-share-link-btn"
          class="q-mr-xs download-logs-btn q-px-sm"
          size="sm"
          icon="share"
          :title="t('search.shareLink')"
          @click="shareLink.execute()"
          :loading="shareLink.isLoading.value"
        ></q-btn>
        <q-btn
          data-test="logs-search-bar-share-link-btn"
          class="q-mr-xs download-logs-btn q-px-sm"
          size="sm"
          icon="history"
          :title="'Search History'"
          @click="showSearchHistoryfn"
        ></q-btn>
        <div class="float-left">
          <date-time
            ref="dateTimeRef"
            auto-apply
            :default-type="searchObj.data.datetime.type"
            :default-absolute-time="{
              startTime: searchObj.data.datetime.startTime,
              endTime: searchObj.data.datetime.endTime,
            }"
            :default-relative-time="searchObj.data.datetime.relativeTimePeriod"
            data-test="logs-search-bar-date-time-dropdown"
            :queryRangeRestrictionMsg="
              searchObj.data.datetime.queryRangeRestrictionMsg
            "
            :queryRangeRestrictionInHour="
              searchObj.data.datetime.queryRangeRestrictionInHour
            "
            @on:date-change="updateDateTime"
            @on:timezone-change="updateTimezone"
            :disable="disable"
          />
        </div>
        <div class="search-time float-left q-mr-xs">
          <div class="flex">
            <auto-refresh-interval
              class="q-mr-xs q-px-none logs-auto-refresh-interval"
              v-model="searchObj.meta.refreshInterval"
              :trigger="true"
              :min-refresh-interval="
                store.state?.zoConfig?.min_auto_refresh_interval ?? 0
              "
              @update:model-value="onRefreshIntervalUpdate"
              @trigger="$emit('onAutoIntervalTrigger')"
            />
            <q-btn-group
              class="no-outline q-pa-none no-border q-mr-xs"
              v-if="
                config.isEnterprise == 'true' &&
                Object.keys(store.state.regionInfo).length > 0 &&
                store.state.zoConfig.super_cluster_enabled
              "
            >
              <q-btn-dropdown
                data-test="logs-search-bar-region-btn"
                class="region-dropdown-btn q-px-xs"
                :title="t('search.regionTitle')"
                label="Region"
              >
                <q-input
                  ref="reginFilterRef"
                  filled
                  flat
                  dense
                  v-model="regionFilter"
                  :label="t('search.regionFilterMsg')"
                >
                  <template v-slot:append>
                    <q-icon
                      v-if="regionFilter !== ''"
                      name="clear"
                      class="cursor-pointer"
                      @click="resetRegionFilter"
                    />
                  </template>
                </q-input>
                <q-tree
                  class="col-12 col-sm-6"
                  :nodes="store.state.regionInfo"
                  node-key="label"
                  :filter="regionFilter"
                  :filter-method="regionFilterMethod"
                  tick-strategy="leaf"
                  v-model:ticked="searchObj.meta.clusters"
                />
              </q-btn-dropdown>
            </q-btn-group>
            <div v-if="searchObj.meta.logsVisualizeToggle === 'visualize'">
              <q-btn
                v-if="
                  config.isEnterprise == 'true' &&
                  visualizeSearchRequestTraceIds.length
                "
                data-test="logs-search-bar-visualize-cancel-btn"
                dense
                flat
                :title="t('search.cancel')"
                class="q-pa-none search-button cancel-search-button"
                @click="cancelVisualizeQueries"
                >{{ t("search.cancel") }}</q-btn
              >
              <q-btn
                v-else
                data-test="logs-search-bar-visualize-refresh-btn"
                dense
                flat
                :title="t('search.runQuery')"
                class="q-pa-none search-button"
                @click="handleRunQueryFn"
                :disable="disable"
                >{{ t("search.runQuery") }}</q-btn
              >
            </div>
            <div v-else>
              <q-btn
                v-if="
                  config.isEnterprise == 'true' &&
                  !!searchObj.data.searchRequestTraceIds.length &&
                  (searchObj.loading == true ||
                    searchObj.loadingHistogram == true)
                "
                data-test="logs-search-bar-refresh-btn"
                data-cy="search-bar-refresh-button"
                dense
                flat
                :title="t('search.cancel')"
                class="q-pa-none search-button cancel-search-button"
                @click="cancelQuery"
                >{{ t("search.cancel") }}</q-btn
              >
              <q-btn
                v-else
                data-test="logs-search-bar-refresh-btn"
                data-cy="search-bar-refresh-button"
                dense
                flat
                :title="t('search.runQuery')"
                class="q-pa-none search-button"
                @click="handleRunQueryFn"
                :disable="
                  searchObj.loading == true ||
                  searchObj.loadingHistogram == true
                "
                >{{ t("search.runQuery") }}</q-btn
              >
            </div>
          </div>
        </div>
      </div>
    </div>

    <div class="row query-editor-container" v-show="searchObj.meta.showQuery">
      <div
        class="col"
        style="border-top: 1px solid #dbdbdb; height: 100%"
        :class="{ 'expand-on-focus': isFocused }"
        :style="backgroundColorStyle"
      >
        <q-splitter
          class="logs-search-splitter"
          no-scroll
          v-model="searchObj.config.fnSplitterModel"
          :limits="searchObj.config.fnSplitterLimit"
          style="width: 100%; height: 100%"
        >
          <template #before>
            <query-editor
              data-test="logs-search-bar-query-editor"
              editor-id="logsQueryEditor"
              ref="queryEditorRef"
              class="monaco-editor"
              :style="editorWidthToggleFunction"
              v-model:query="searchObj.data.query"
              :keywords="autoCompleteKeywords"
              :suggestions="autoCompleteSuggestions"
              @keydown.ctrl.enter="handleRunQueryFn"
              @update:query="updateQueryValue"
              @run-query="handleRunQueryFn"
              :class="
                searchObj.data.editorValue == '' &&
                searchObj.meta.queryEditorPlaceholderFlag
                  ? 'empty-query'
                  : ''
              "
              @focus="searchObj.meta.queryEditorPlaceholderFlag = false"
              @blur="searchObj.meta.queryEditorPlaceholderFlag = true"
            />
          </template>
          <template #after>
            <div
              data-test="logs-vrl-function-editor"
              v-show="searchObj.meta.toggleFunction"
              style="width: 100%; height: 100%"
            >
              <query-editor
                data-test="logs-vrl-function-editor"
                ref="fnEditorRef"
                editor-id="fnEditor"
                class="monaco-editor"
                v-model:query="searchObj.data.tempFunctionContent"
                :class="
                  searchObj.data.tempFunctionContent == '' &&
                  searchObj.meta.functionEditorPlaceholderFlag
                    ? 'empty-function'
                    : ''
                "
                @keydown.ctrl.enter="handleRunQueryFn"
                language="vrl"
                @focus="searchObj.meta.functionEditorPlaceholderFlag = false"
                @blur="searchObj.meta.functionEditorPlaceholderFlag = true"
              />
            </div>
          </template>
        </q-splitter>
      </div>
      <q-btn
        data-test="logs-query-editor-full_screen-btn"
        :icon="isFocused ? 'fullscreen_exit' : 'fullscreen'"
        :title="isFocused ? 'Collapse' : 'Expand'"
        dense
        size="10px"
        round
        color="primary"
        @click="isFocused = !isFocused"
        style="position: absolute; top: 42px; right: 10px; z-index: 20"
      ></q-btn>
    </div>

    <q-dialog ref="confirmDialog" v-model="confirmDialogVisible">
      <q-card>
        <q-card-section>
          {{ confirmMessage }}
        </q-card-section>

        <q-card-actions align="right">
          <q-btn
            data-test="logs-search-bar-confirm-dialog-cancel-btn"
            :label="t('confirmDialog.cancel')"
            color="primary"
            @click="cancelConfirmDialog"
          />
          <q-btn
            data-test="logs-search-bar-confirm-dialog-ok-btn"
            :label="t('confirmDialog.ok')"
            color="positive"
            @click="confirmDialogOK"
          />
        </q-card-actions>
      </q-card>
    </q-dialog>
    <q-dialog
      ref="confirmSavedViewDialog"
      v-model="confirmSavedViewDialogVisible"
    >
      <q-card>
        <q-card-section>
          {{ confirmMessageSavedView }}
        </q-card-section>

        <q-card-actions align="right">
          <q-btn
            data-test="logs-search-bar-confirm-dialog-cancel-btn"
            :label="t('confirmDialog.cancel')"
            color="primary"
            @click="cancelConfirmDialog"
          />
          <q-btn
            data-test="logs-search-bar-confirm-dialog-ok-btn"
            :label="t('confirmDialog.ok')"
            color="positive"
            @click="confirmDialogOK"
          />
        </q-card-actions>
      </q-card>
    </q-dialog>
    <q-dialog v-model="customDownloadDialog">
      <q-card>
        <q-card-section>
          {{ t("search.customDownloadMessage") }}
        </q-card-section>

        <q-card-section>
          <q-input
            type="number"
            data-test="custom-download-initial-number-input"
            v-model="downloadCustomInitialNumber"
            :label="t('search.initialNumber')"
            default-value="1"
            color="input-border"
            bg-color="input-bg"
            class="showLabelOnTop"
            stack-label
            outlined
            filled
            dense
            tabindex="0"
            min="1"
          />
          <q-select
            data-test="custom-download-range-select"
            v-model="downloadCustomRange"
            :options="downloadCustomRangeOptions"
            :label="t('search.range')"
            color="input-border"
            bg-color="input-bg"
            class="q-py-sm showLabelOnTop"
            stack-label
            outlined
            filled
            dense
          />
        </q-card-section>

        <q-card-actions align="right">
          <q-btn
            unelevated
            no-caps
            class="q-mr-sm text-bold"
            data-test="logs-search-bar-confirm-dialog-cancel-btn"
            :label="t('confirmDialog.cancel')"
            color="secondary"
            v-close-popup
          />
          <q-btn
            unelevated
            no-caps
            class="q-mr-sm text-bold"
            data-test="logs-search-bar-confirm-dialog-ok-btn"
            :label="t('search.btnDownload')"
            color="primary"
            @click="downloadRangeData"
          />
        </q-card-actions>
      </q-card>
    </q-dialog>
    <q-dialog v-model="store.state.savedViewDialog">
      <q-card style="width: 700px; max-width: 80vw">
        <q-card-section>
          <div class="text-h6">{{ t("search.savedViewsLabel") }}</div>
        </q-card-section>

        <q-card-section class="q-pt-none">
          <div v-if="isSavedViewAction == 'create'">
            <q-input
              data-test="add-alert-name-input"
              v-model="savedViewName"
              :label="t('search.savedViewName')"
              color="input-border"
              bg-color="input-bg"
              class="showLabelOnTop"
              stack-label
              outlined
              filled
              dense
              :rules="[
                (val) => !!val.trim() || 'This field is required',
                (val) =>
                  /^[-A-Za-z0-9 /@/_]+$/.test(val) ||
                  'Input must be alphanumeric',
              ]"
              tabindex="0"
            />
          </div>
          <div v-else>
            <q-select
              data-test="saved-view-name-select"
              v-model="savedViewSelectedName"
              :options="searchObj.data.savedViews"
              option-label="view_name"
              option-value="view_id"
              :label="t('search.savedViewName')"
              :popup-content-style="{ textTransform: 'capitalize' }"
              color="input-border"
              bg-color="input-bg"
              class="q-py-sm showLabelOnTop"
              stack-label
              outlined
              filled
              dense
              :rules="[(val: any) => !!val || 'Field is required!']"
            />
          </div>
        </q-card-section>

        <q-card-actions align="right" class="bg-white text-teal">
          <q-btn
            data-test="saved-view-dialog-cancel-btn"
            unelevated
            no-caps
            class="q-mr-sm text-bold"
            :label="t('confirmDialog.cancel')"
            color="secondary"
            v-close-popup
          />
          <q-btn
            data-test="saved-view-dialog-save-btn"
            v-if="!saveViewLoader"
            unelevated
            no-caps
            :label="t('common.save')"
            color="primary"
            class="text-bold"
            @click="handleSavedView"
          />
          <q-btn
            data-test="saved-view-dialog-loading-btn"
            v-if="saveViewLoader"
            unelevated
            no-caps
            :label="t('confirmDialog.loading')"
            color="primary"
            class="text-bold"
          />
        </q-card-actions>
      </q-card>
    </q-dialog>
    <q-dialog v-model="store.state.savedFunctionDialog">
      <q-card style="width: 700px; max-width: 80vw">
        <q-card-section>
          <div class="text-h6">{{ t("search.functionPlaceholder") }}</div>
        </q-card-section>

        <q-card-section class="q-pt-none">
          <span>Update</span>
          <q-toggle
            data-test="saved-function-action-toggle"
            v-bind:disable="functionOptions.length == 0"
            name="saved_function_action"
            v-model="isSavedFunctionAction"
            true-value="create"
            false-value="update"
            label=""
            @change="savedFunctionName = ''"
          />
          <span>Create</span>
          <div v-if="isSavedFunctionAction == 'create'">
            <q-input
              data-test="saved-function-name-input"
              v-model="savedFunctionName"
              :label="t('search.saveFunctionName')"
              color="input-border"
              bg-color="input-bg"
              class="showLabelOnTop"
              stack-label
              outlined
              filled
              dense
              :rules="[
                (val) => !!val.trim() || 'This field is required',
                (val) =>
                  /^[-A-Za-z0-9/_]+$/.test(val) || 'Input must be alphanumeric',
              ]"
              tabindex="0"
            />
          </div>
          <div v-else>
            <q-select
              data-test="saved-function-name-select"
              v-model="savedFunctionSelectedName"
              :options="functionOptions"
              option-label="name"
              option-value="name"
              :label="t('search.saveFunctionName')"
              placeholder="Select Function Name"
              :popup-content-style="{ textTransform: 'capitalize' }"
              color="input-border"
              bg-color="input-bg"
              class="q-py-sm showLabelOnTop"
              stack-label
              outlined
              filled
              dense
              :rules="[(val: any) => !!val || 'Field is required!']"
            />
          </div>
        </q-card-section>

        <q-card-actions align="right" class="bg-white text-teal">
          <q-btn
            data-test="saved-function-dialog-cancel-btn"
            unelevated
            no-caps
            class="q-mr-sm text-bold"
            :label="t('confirmDialog.cancel')"
            color="secondary"
            v-close-popup
          />
          <q-btn
            data-test="saved-view-dialog-save-btn"
            v-if="!saveFunctionLoader"
            unelevated
            no-caps
            :label="t('confirmDialog.ok')"
            color="primary"
            class="text-bold"
            @click="saveFunction"
          />
          <q-btn
            data-test="saved-function-dialog-loading-btn"
            v-if="saveFunctionLoader"
            unelevated
            no-caps
            :label="t('confirmDialog.loading')"
            color="primary"
            class="text-bold"
          />
        </q-card-actions>
      </q-card>
    </q-dialog>
    <q-dialog v-model="searchSchedulerJob">
      <q-card style="width: 700px; max-width: 80vw">
        <q-card-section>
          <div class="text-h6">Schedule Search Job</div>
        </q-card-section>

        <q-card-section class="q-pt-none">
          <div>
            <div class="text-left q-mb-xs">
                No of Records:
                <q-icon
            name="info"
            size="17px"
            class="q-ml-xs cursor-pointer"

          >
            <q-tooltip
              anchor="center right"
              self="center left"
              max-width="300px"
            >
              <span style="font-size: 14px"
                >Number of records can be specified eg: if the no. of records is 1000 then user can get maximum of 1000 records</span
              >
            </q-tooltip>
          </q-icon>
            </div>
            <q-input
              type="number"
              data-test="search-scheuduler-max-number-of-records-input"
              v-model="searchObj.meta.jobRecords"
              default-value="100"
              color="input-border"
              bg-color="input-bg"
              class="showLabelOnTop"
              stack-label
              outlined
              filled
              dense
              tabindex="0"
              min="100"
          />
          
          </div>
          <div class="text-left">
            Maximum 100000 events can be returned in schedule job
          </div>
          <div style="opacity: 0.8;" class="text-left mapping-warning-msg q-mt-md">
                    <q-icon name="warning" color="red" class="q-mr-sm" />
                    <span>Histogram will be disabled for the schedule job</span>
                    </div>
        </q-card-section>
           

        <q-card-actions align="right" class=" text-teal">
          <q-btn
            data-test="search-scheduler-max-records-cancel-btn"
            unelevated
            no-caps
            class="q-mr-sm text-bold"
            :label="t('confirmDialog.cancel')"
            color="secondary"
            v-close-popup
            @click="{
              searchSchedulerJob = false;
              searchObj.meta.showSearchScheduler = false;
            }"
          />
          <q-btn
            data-test="search-scheduler-max-records-submit-btn"
            unelevated
            no-caps
            :label="t('confirmDialog.ok')"
            color="primary"
            class="text-bold"
            @click="addJobScheduler"
            v-close-popup
          />
        </q-card-actions>
      </q-card>
    </q-dialog>
    <ConfirmDialog
      title="Delete Saved View"
      message="Are you sure you want to delete saved view?"
      @update:ok="confirmDeleteSavedViews"
      @update:cancel="confirmDelete = false"
      v-model="confirmDelete"
    />
    <ConfirmDialog
      title="Update Saved View"
      message="Are you sure you want to update the saved view? This action will overwrite the existing one."
      @update:ok="confirmUpdateSavedViews"
      @update:cancel="confirmUpdate = false"
      v-model="confirmUpdate"
    />
    <ConfirmDialog
      title="Reset Changes"
      message="Navigating away from visualize will reset your changes. Are you sure you want to proceed?"
      @update:ok="changeLogsVisualizeToggle"
      @update:cancel="confirmLogsVisualizeModeChangeDialog = false"
      v-model="confirmLogsVisualizeModeChangeDialog"
    />
  </div>
</template>

<script lang="ts">
// @ts-nocheck
import {
  defineComponent,
  ref,
  onMounted,
  nextTick,
  watch,
  toRaw,
  onActivated,
  onUnmounted,
  onDeactivated,
  defineAsyncComponent,
  onBeforeMount,
} from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { useStore } from "vuex";
import { useQuasar, copyToClipboard, is } from "quasar";

import DateTime from "@/components/DateTime.vue";
import useLogs from "@/composables/useLogs";
import SyntaxGuide from "./SyntaxGuide.vue";
import jsTransformService from "@/services/jstransform";
import searchService from "@/services/search";
import shortURLService from "@/services/short_url";

import segment from "@/services/segment_analytics";
import config from "@/aws-exports";

import AutoRefreshInterval from "@/components/AutoRefreshInterval.vue";
import useSqlSuggestions from "@/composables/useSuggestions";
import {
  mergeDeep,
  b64DecodeUnicode,
  getImageURL,
  useLocalInterestingFields,
  useLocalSavedView,
  queryIndexSplit,
  timestampToTimezoneDate,
  b64EncodeUnicode,
} from "@/utils/zincutils";

import savedviewsService from "@/services/saved_views";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import { cloneDeep } from "lodash-es";
import useDashboardPanelData from "@/composables/useDashboardPanel";
import { inject } from "vue";
import QueryEditor from "@/components/QueryEditor.vue";
import useCancelQuery from "@/composables/dashboard/useCancelQuery";
import { computed } from "vue";
import { useLoading } from "@/composables/useLoading";
const defaultValue: any = () => {
  return {
    name: "",
    function: "",
    params: "row",
    transType: "0",
  };
};

export default defineComponent({
  name: "ComponentSearchSearchBar",
  components: {
    DateTime,
    QueryEditor,
    SyntaxGuide,
    AutoRefreshInterval,
    ConfirmDialog,  },
  emits: [
    "searchdata",
    "onChangeInterval",
    "onChangeTimezone",
    "handleQuickModeChange",
    "handleRunQueryFn",
    "onAutoIntervalTrigger",
    "showSearchHistory",
  ],
  methods: {
    searchData() {
      if (this.searchObj.loading == false) {
        // this.searchObj.runQuery = true;
        this.$emit("searchdata");
      }
    },
    changeFunctionName(value) {
      // alert(value)
      // console.log(value);
    },
    createNewValue(inputValue, doneFn) {
      // Call the doneFn with the new value
      doneFn(inputValue);
    },
    updateSelectedValue() {
      // Update the selected value with the newly created value
      if (
        this.functionModel &&
        !this.functionOptions.includes(this.functionModel)
      ) {
        this.functionOptions.push(this.functionModel);
      }
    },
    handleDeleteSavedView(item: any) {
      this.savedViewDropdownModel = false;
      this.deleteViewID = item.view_id;
      this.confirmDelete = true;
    },
    handleUpdateSavedView(item: any) {
      if (this.searchObj.data.stream.selectedStream.length == 0) {
        this.$q.notify({
          type: "negative",
          message: "No stream available to update save view.",
        });
        return;
      }
      this.savedViewDropdownModel = false;
      this.updateViewObj = item;
      this.confirmUpdate = true;
    },
    confirmDeleteSavedViews() {
      this.deleteSavedViews();
    },
    toggleCustomDownloadDialog() {
      this.customDownloadDialog = true;
    },
    confirmUpdateSavedViews() {
      this.updateSavedViews(
        this.updateViewObj.view_id,
        this.updateViewObj.view_name,
      );
      return;
    },
    downloadRangeData() {
      let initNumber = parseInt(this.downloadCustomInitialNumber);
      if (initNumber < 0) {
        this.$q.notify({
          message: "Initial number must be positive number.",
          color: "negative",
          position: "bottom",
          timeout: 2000,
        });
        return;
      }
      // const queryReq = this.buildSearch();
      this.searchObj.data.customDownloadQueryObj.query.from = initNumber;
      this.searchObj.data.customDownloadQueryObj.query.size =
        this.downloadCustomRange;
      searchService
        .search(
          {
            org_identifier: this.searchObj.organizationIdentifier,
            query: this.searchObj.data.customDownloadQueryObj,
            page_type: this.searchObj.data.stream.streamType,
          },
          "UI",
        )
        .then((res) => {
          this.customDownloadDialog = false;
          if (res.data.hits.length > 0) {
            this.downloadLogs(res.data.hits);
          } else {
            this.$q.notify({
              message: "No data found to download.",
              color: "positive",
              position: "top",
              timeout: 2000,
            });
          }
        })
        .catch((err) => {
          this.$q.notify({
            message: err.message,
            color: "negative",
            position: "top",
            timeout: 2000,
          });
        });
    },
  },
  props: {
    fieldValues: {
      type: Object,
      default: () => ({}),
    },
  },
  setup(props, { emit }) {
    const router = useRouter();
    const { t } = useI18n();
    const $q = useQuasar();
    const store = useStore();
    const rowsPerPage = ref(10);
    const regionFilter = ref();
    const regionFilterRef = ref(null);

    const {
      searchObj,
      refreshData,
      handleRunQuery,
      updatedLocalLogFilterField,
      getSavedViews,
      getQueryData,
      getStreams,
      updateUrlQueryParams,
      generateURLQuery,
      buildSearch,
      resetStreamData,
      loadStreamLists,
      fnParsedSQL,
      fnUnparsedSQL,
      onStreamChange,
      moveItemsToTop,
      validateFilterForMultiStream,
      extractFields,
      cancelQuery,
      setSelectedStreams,
      getJobData,
      routeToSearchSchedule,
    } = useLogs();
    const queryEditorRef = ref(null);

    const formData: any = ref(defaultValue());
    const functionOptions = ref(searchObj.data.transforms);

    const functionModel: string = ref(null);
    const fnEditorRef: any = ref(null);

    const isSavedFunctionAction: string = ref("create");
    const savedFunctionName: string = ref("");
    const savedFunctionSelectedName: string = ref("");
    const saveFunctionLoader = ref(false);

    const isFocused = ref(false);

    // confirm dialog for logs visualization toggle
    const confirmLogsVisualizeModeChangeDialog = ref(false);

    const confirmDialogVisible: boolean = ref(false);
    const confirmSavedViewDialogVisible: boolean = ref(false);
    const searchSchedulerJob = ref(false);
    const autoSearchSchedulerJob = ref(false);
    let confirmCallback;
    let streamName = "";

    let parser: any;
    const dateTimeRef = ref(null);
    const saveViewLoader = ref(false);
    const favoriteViews = ref([]);

    const localSavedViews = ref([]);
    let savedViews = useLocalSavedView();
    if (savedViews.value != null) {
      const favoriteValues = [];
      Object.values(savedViews.value).forEach((view) => {
        if (view.org_id === store.state.selectedOrganization.identifier) {
          favoriteViews.value.push(view.view_id);
          favoriteValues.push(view);
        }
      });

      localSavedViews.value.push(...favoriteValues);
    }

    const {
      autoCompleteData,
      autoCompleteKeywords,
      autoCompleteSuggestions,
      getSuggestions,
      updateFieldKeywords,
      updateFunctionKeywords,
    } = useSqlSuggestions();

    const refreshTimeChange = (item) => {
      searchObj.meta.refreshInterval = Number(item.value);
    };

    const isSavedViewAction = ref("create");
    const savedViewName = ref("");
    const savedViewSelectedName = ref("");
    const confirmDelete = ref(false);
    const deleteViewID = ref("");
    const savedViewDropdownModel = ref(false);
    const searchTerm = ref(""); 
    const filteredFunctionOptions = computed(() => {
      if (!searchTerm.value) return functionOptions.value;
      return functionOptions.value.filter((item) =>
        item.name.toLowerCase().includes(searchTerm.value.toLowerCase())
      );
    });
    const confirmUpdate = ref(false);
    const updateViewObj = ref({});

    watch(
      () => searchObj.data.stream.selectedStreamFields,
      (fields) => {
        if (fields != undefined && fields.length) updateFieldKeywords(fields);
      },
      { immediate: true, deep: true },
    );
    watch(
      () => searchObj.meta.showSearchScheduler,
      (showSearchScheduler) => {
        if(showSearchScheduler){
          searchSchedulerJob.value = true;
        }
      },
      { immediate: true, deep: true },
    );
    watch(
      () => searchObj.meta.functionEditorPlaceholderFlag,
      (val) => {
        if (
          searchObj.meta.jobId != "" &&
          val == true &&
          (router.currentRoute.value.query.functionContent || searchObj.data.tempFunctionContent != "")
        ) {
          if (!checkFnQuery(searchObj.data.tempFunctionContent)) {
            $q.notify({
              message: "Job Context have been removed",
              color: "warning",
              position: "bottom",
              timeout: 2000,
            });
            searchObj.meta.jobId = "";
            searchObj.data.queryResults.hits = [];
            // getQueryData(false);
          }
        }
      },
      { immediate: true, deep: true },
    );
    watch(
      () => searchObj.meta.showHistogram,
      (val) => {
        
        if(val == true && searchObj.meta.jobId != ""  ){
          $q.notify({
              message: "Histogram is not available for scheduled search",
              color: "negative",
              position: "bottom",
              timeout: 2000,
            });
            searchObj.meta.showHistogram = false;
            searchObj.loadingHistogram = false;
        }
        
      },
      { immediate: true, deep: true },
    );
    watch(
      () => searchObj.data.stream.functions,
      (funs) => {
        if (funs.length) updateFunctionKeywords(funs);
      },
      { immediate: true, deep: true },
    );


    onBeforeMount(async () => {
      await importSqlParser();
    });

    const importSqlParser = async () => {
      const useSqlParser: any = await import("@/composables/useParser");
      const { sqlParser }: any = useSqlParser.default();
      parser = await sqlParser();
    };

    const updateAutoComplete = (value) => {
      autoCompleteData.value.query = value;
      autoCompleteData.value.cursorIndex =
        queryEditorRef?.value?.getCursorIndex();
      autoCompleteData.value.fieldValues = props.fieldValues;
      autoCompleteData.value.popup.open =
        queryEditorRef?.value?.triggerAutoComplete;
      getSuggestions();
    };

    const getColumnNames = (parsedSQL: any) => {
      const columnData = parsedSQL?.columns;
      let columnNames = [];
      for (const item of columnData) {
        if (item.expr.type === "column_ref") {
          columnNames.push(item.expr.column?.expr?.value);
        } else if (item.expr.type === "aggr_func") {
          if (item.expr.args?.expr?.hasOwnProperty("column")) {
            columnNames.push(item.expr.args?.expr?.column?.value);
          } else if (item.expr.args?.expr?.value) {
            columnNames.push(item.expr.args?.expr?.value);
          }
        } else if (item.expr.type === "function") {
          item.expr.args.value.map((val) => {
            if (val.type === "column_ref") {
              columnNames.push(val.column?.expr?.value);
            }
          });
        }
      }

      if (parsedSQL?._next) {
        columnNames = getColumnNames(parsedSQL._next);
      }
      return columnNames;
    };

    const updateQueryValue = (value: string) => {
      // if (searchObj.meta.jobId != "") {
      //   searchObj.meta.jobId = "";
      //   getQueryData(false);
      // }


      searchObj.data.editorValue = value;
      if (searchObj.meta.quickMode === true) {
        const parsedSQL = fnParsedSQL();
        if (
          searchObj.meta.sqlMode === true &&
          Object.hasOwn(parsedSQL, "from")
        ) {
          setSelectedStreams(value);
          // onStreamChange(value);
        }
        if (parsedSQL != undefined && parsedSQL?.columns?.length > 0) {
          const columnNames = getColumnNames(parsedSQL);

          searchObj.data.stream.interestingFieldList = [];
          for (const col of columnNames) {
            if (
              !searchObj.data.stream.interestingFieldList.includes(col) &&
              col != "*"
            ) {
              // searchObj.data.stream.interestingFieldList.push(col);
              const localInterestingFields: any = useLocalInterestingFields();
              let localFields: any = {};
              if (localInterestingFields.value != null) {
                localFields = localInterestingFields.value;
              }
              for (const stream of searchObj.data.stream.selectedStreamFields) {
                if (
                  stream.name == col &&
                  !searchObj.data.stream.interestingFieldList.includes(col)
                ) {
                  searchObj.data.stream.interestingFieldList.push(col);
                  localFields[
                    searchObj.organizationIdentifier +
                      "_" +
                      searchObj.data.stream.selectedStream[0]
                  ] = searchObj.data.stream.interestingFieldList;
                }
              }
              useLocalInterestingFields(localFields);
            }
          }

          for (const item of searchObj.data.stream.selectedStreamFields) {
            if (
              searchObj.data.stream.interestingFieldList.includes(item.name)
            ) {
              item.isInterestingField = true;
            } else {
              item.isInterestingField = false;
            }
          }
        }
      }

      if (value != "" && searchObj.meta.sqlMode === true) {
        const parsedSQL = fnParsedSQL();
        if (
          Object.hasOwn(parsedSQL, "from") ||
          Object.hasOwn(parsedSQL, "select")
        ) {
          setSelectedStreams(value);
          // onStreamChange(value);
        }
      }

      updateAutoComplete(value);
      try {
        if (searchObj.meta.sqlMode === true) {
          searchObj.data.parsedQuery = parser.astify(value);
          if (searchObj.data.parsedQuery?.from?.length > 0) {
            const tableName: string =
              searchObj.data.parsedQuery.from[0].table ||
              searchObj.data.parsedQuery.from[0].expr?.ast?.from?.[0]?.table;
            if (
              !searchObj.data.stream.selectedStream.includes(tableName) &&
              tableName !== streamName
            ) {
              let streamFound = false;
              searchObj.data.stream.selectedStream = [];

              streamName = tableName;
              searchObj.data.streamResults.list.forEach((stream) => {
                if (stream.name == streamName) {
                  streamFound = true;
                  let itemObj = {
                    label: stream.name,
                    value: stream.name,
                  };

                  // searchObj.data.stream.selectedStream = itemObj;
                  searchObj.data.stream.selectedStream.push(itemObj.value);
                  onStreamChange(searchObj.data.query);
                }
              });
              if (streamFound == false) {
                // searchObj.data.stream.selectedStream = { label: "", value: "" };
                searchObj.data.stream.selectedStream = [];
                searchObj.data.stream.selectedStreamFields = [];
                $q.notify({
                  message: "Stream not found",
                  color: "negative",
                  position: "top",
                  timeout: 2000,
                });
              }
            }
          }
        }
        //here we reset the job id if user change the query and move outside of the editor
        if (
          searchObj.meta.jobId != "" &&
          searchObj.meta.queryEditorPlaceholderFlag == true
        ) {
          if (!checkQuery(value)) {
            $q.notify({
              message: "Job Context have been removed",
              color: "warning",
              position: "bottom",
              timeout: 2000,
            });
            searchObj.meta.jobId = "";
            searchObj.data.queryResults.hits = [];
            // getQueryData(false);
          }
       }
      } catch (e) {
        console.log(e, "Logs: Error while updating query value");
      }

    };
    const handleEscKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        isFocused.value = false;
      }
    };

    const updateDateTime = async (value: object) => {
      if (
        value.valueType == "absolute" &&
        searchObj.data.stream.selectedStream.length > 0 &&
        searchObj.data.datetime.queryRangeRestrictionInHour > 0 &&
        value.hasOwnProperty("selectedDate") &&
        value.hasOwnProperty("selectedTime") &&
        value.selectedDate.hasOwnProperty("from") &&
        value.selectedTime.hasOwnProperty("startTime")
      ) {
        // Convert hours to microseconds
        let newStartTime =
          parseInt(value.endTime) -
          searchObj.data.datetime.queryRangeRestrictionInHour *
            60 *
            60 *
            1000000;

        if (parseInt(newStartTime) > parseInt(value.startTime)) {
          value.startTime = newStartTime;

          value.selectedDate.from = timestampToTimezoneDate(
            value.startTime / 1000,
            store.state.timezone,
            "yyyy/MM/DD",
          );
          value.selectedTime.startTime = timestampToTimezoneDate(
            value.startTime / 1000,
            store.state.timezone,
            "HH:mm",
          );

          dateTimeRef.value.setAbsoluteTime(value.startTime, value.endTime);
          dateTimeRef.value.setDateType("absolute");
        }
      }
      searchObj.data.datetime = {
        startTime: value.startTime,
        endTime: value.endTime,
        relativeTimePeriod: value.relativeTimePeriod
          ? value.relativeTimePeriod
          : searchObj.data.datetime.relativeTimePeriod,
        type: value.relativeTimePeriod ? "relative" : "absolute",
        selectedDate: value?.selectedDate,
        selectedTime: value?.selectedTime,
        queryRangeRestrictionMsg:
          searchObj.data.datetime?.queryRangeRestrictionMsg || "",
        queryRangeRestrictionInHour:
          searchObj.data.datetime?.queryRangeRestrictionInHour || 0,
      };

      await nextTick();
      await nextTick();
      await nextTick();
      await nextTick();

      if (
        searchObj.loading == false &&
        store.state.zoConfig.query_on_stream_selection == false
      ) {
        searchObj.loading = true;
        searchObj.runQuery = true;
      }

      if (config.isCloud == "true" && value.userChangedValue) {
        segment.track("Button Click", {
          button: "Date Change",
          tab: value.tab,
          value: value,
          //user_org: this.store.state.selectedOrganization.identifier,
          //user_id: this.store.state.userInfo.email,
          stream_name: searchObj.data.stream.selectedStream.join(","),
          page: "Search Logs",
        });
      }

      if (
        value.valueType === "relative" &&
        store.state.zoConfig.query_on_stream_selection == false
      ) {
        emit("searchdata");
      }
    };

    const updateTimezone = () => {
      if (store.state.zoConfig.query_on_stream_selection == false) {
        emit("onChangeTimezone");
      }
    };

    const updateQuery = () => {
      if (queryEditorRef.value?.setValue)
        queryEditorRef.value.setValue(searchObj.data.query);
    };

    const jsonToCsv = (jsonData) => {
      const replacer = (key, value) => (value === null ? "" : value);
      const header = Object.keys(jsonData[0]);
      let csv = header.join(",") + "\r\n";

      for (let i = 0; i < jsonData.length; i++) {
        const row = header
          .map((fieldName) => JSON.stringify(jsonData[i][fieldName], replacer))
          .join(",");
        csv += row + "\r\n";
      }

      return csv;
    };

    const downloadLogs = (data) => {
      const filename = "logs-data.csv";
      const dataobj = jsonToCsv(data);
      const file = new File([dataobj], filename, {
        type: "text/csv",
      });
      const url = URL.createObjectURL(file);
      const link = document.createElement("a");
      link.href = url;
      link.download = filename;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      URL.revokeObjectURL(url);
    };

    onMounted(async () => {
      if (
        router.currentRoute.value.query.functionContent ||
        searchObj.data.tempFunctionContent
      ) {
        searchObj.meta.toggleFunction = true;
        const fnContent = router.currentRoute.value.query.functionContent
          ? b64DecodeUnicode(router.currentRoute.value.query.functionContent)
          : searchObj.data.tempFunctionContent;
        fnEditorRef?.value?.setValue(fnContent);
        fnEditorRef?.value?.resetEditorLayout();
        searchObj.config.fnSplitterModel = 60;
      }
      window.addEventListener("keydown", handleEscKey);
    });

    onUnmounted(() => {
      window.removeEventListener("click", () => {
        fnEditorRef?.value?.resetEditorLayout();
      });
      window.removeEventListener("keydown", handleEscKey);
    });

    onActivated(() => {
      updateQuery();

      if (
        router.currentRoute.value.query.functionContent ||
        searchObj.data.tempFunctionContent
      ) {
        searchObj.meta.toggleFunction = true;
        const fnContent = router.currentRoute.value.query.functionContent
          ? b64DecodeUnicode(router.currentRoute.value.query.functionContent)
          : searchObj.data.tempFunctionContent;
        fnEditorRef?.value?.setValue(fnContent);
        fnEditorRef?.value?.resetEditorLayout();
        searchObj.config.fnSplitterModel = 60;
        window.removeEventListener("click", () => {
          fnEditorRef?.value?.resetEditorLayout();
        });
      }
      fnEditorRef?.value?.resetEditorLayout();
    });

    onDeactivated(() => {
      window.removeEventListener("click", () => {
        fnEditorRef?.value?.resetEditorLayout();
      });
    });

    const saveFunction = () => {
      saveFunctionLoader.value = true;
      let callTransform: Promise<{ data: any }>;
      const content = searchObj.data.tempFunctionContent;
      let fnName = "";
      if (isSavedFunctionAction.value == "create") {
        fnName = savedFunctionName.value;
      } else {
        fnName = savedFunctionSelectedName.value.name;
      }

      if (content.trim() == "") {
        $q.notify({
          type: "warning",
          message:
            "The function field must contain a value and cannot be left empty.",
        });
        saveFunctionLoader.value = false;
        return;
      }

      const pattern = /^[a-zA-Z][a-zA-Z0-9_]*$/;
      if (!pattern.test(fnName)) {
        $q.notify({
          type: "negative",
          message: "Function name is not valid.",
        });
        saveFunctionLoader.value = false;
        return;
      }

      formData.value.params = "row";
      formData.value.function = content;
      formData.value.transType = 0;
      formData.value.name = fnName;
      searchObj.data.tempFunctionContent = content;

      // const result = functionOptions.value.find((obj) => obj.name === fnName);
      if (isSavedFunctionAction.value == "create") {
        callTransform = jsTransformService.create(
          store.state.selectedOrganization.identifier,
          formData.value,
        );

        callTransform
          .then((res: { data: any }) => {
            $q.notify({
              type: "positive",
              message: res.data.message,
            });

            functionModel.value = {
              name: formData.value.name,
              function: formData.value.function,
            };
            functionOptions.value.push({
              name: formData.value.name,
              function: formData.value.function,
              transType: 0,
              params: "row",
            });
            store.dispatch("setSavedFunctionDialog", false);
            isSavedFunctionAction.value = "create";
            savedFunctionName.value = "";
            saveFunctionLoader.value = false;
            savedFunctionSelectedName.value = "";
          })
          .catch((err) => {
            saveFunctionLoader.value = false;
            $q.notify({
              type: "negative",
              message:
                JSON.stringify(err.response.data["message"]) ||
                "Function creation failed",
              timeout: 5000,
            });
          });
      } else {
        saveFunctionLoader.value = false;
        showConfirmDialog(() => {
          saveFunctionLoader.value = true;
          callTransform = jsTransformService.update(
            store.state.selectedOrganization.identifier,
            formData.value,
          );

          callTransform
            .then((res: { data: any }) => {
              $q.notify({
                type: "positive",
                message: "Function updated successfully.",
              });

              const transformIndex = searchObj.data.transforms.findIndex(
                (obj) => obj.name === formData.value.name,
              );
              if (transformIndex !== -1) {
                searchObj.data.transforms[transformIndex].name =
                  formData.value.name;
                searchObj.data.transforms[transformIndex].function =
                  formData.value.function;
              }

              functionOptions.value = searchObj.data.transforms;
              store.dispatch("setSavedFunctionDialog", false);
              isSavedFunctionAction.value = "create";
              savedFunctionName.value = "";
              saveFunctionLoader.value = false;
              savedFunctionSelectedName.value = "";
            })
            .catch((err) => {
              saveFunctionLoader.value = false;
              $q.notify({
                type: "negative",
                message:
                  JSON.stringify(err.response.data["message"]) ||
                  "Function updation failed",
                timeout: 5000,
              });
            });
        });
      }
    };

    const resetFunctionContent = () => {
      fnEditorRef.value.setValue("");
      store.dispatch("setSavedFunctionDialog", false);
      isSavedFunctionAction.value = "create";
      savedFunctionName.value = "";
      saveFunctionLoader.value = false;
      savedFunctionSelectedName.value = "";
    };

    const resetEditorLayout = () => {
      setTimeout(() => {
        queryEditorRef?.value?.resetEditorLayout();
        fnEditorRef?.value?.resetEditorLayout();
      }, 100);
    };

    const populateFunctionImplementation = (fnValue, flag = false) => {
      if (flag) {
        $q.notify({
          type: "positive",
          message: `${fnValue.name} function applied successfully.`,
          timeout: 3000,
        });
      }
      searchObj.config.fnSplitterModel = 60;
      fnEditorRef.value.setValue(fnValue.function);
      searchObj.data.tempFunctionName = fnValue.name;
      searchObj.data.tempFunctionContent = fnValue.function;
    };

    const fnSavedFunctionDialog = () => {
      const content = searchObj.data.tempFunctionContent;
      if (content == "") {
        $q.notify({
          type: "negative",
          message: "No function definition found.",
        });
        return;
      }
      store.dispatch("setSavedFunctionDialog", true);
      isSavedFunctionAction.value = "create";
      savedFunctionName.value = "";
      saveFunctionLoader.value = false;
      savedFunctionSelectedName.value = "";
    };

    const showConfirmDialog = (callback) => {
      confirmDialogVisible.value = true;
      confirmCallback = callback;
    };

    const showSavedViewConfirmDialog = (callback) => {
      confirmSavedViewDialogVisible.value = true;
      confirmCallback = callback;
    };

    const cancelConfirmDialog = () => {
      confirmSavedViewDialogVisible.value = false;
      confirmDialogVisible.value = false;
      confirmCallback = null;
    };

    const confirmDialogOK = () => {
      if (confirmCallback) {
        confirmCallback();
      }
      confirmDialogVisible.value = false;
      confirmCallback = null;
    };

    const filterFn = (val, update) => {
      update(() => {
        if (val === "") {
          functionOptions.value = searchObj.data.transforms;
        } else {
          const needle = val.toLowerCase();
          functionOptions.value = searchObj.data.transforms.filter(
            (v) => v.name?.toLowerCase().indexOf(needle) > -1,
          );
        }
      });
    };

    const onRefreshIntervalUpdate = () => {
      emit("onChangeInterval");
    };

    const fnSavedView = () => {
      if (searchObj.data.stream.selectedStream.length == 0) {
        $q.notify({
          type: "negative",
          message: "No stream available to save view.",
        });
        return;
      }
      store.dispatch("setSavedViewDialog", true);
      isSavedViewAction.value = "create";
      savedViewName.value = "";
      saveViewLoader.value = false;
      savedViewSelectedName.value = "";
      savedViewDropdownModel.value = false;
    };

    const applySavedView = (item) => {
      searchObj.shouldIgnoreWatcher = true;
      searchObj.meta.sqlMode = false;
      savedviewsService
        .getViewDetail(
          store.state.selectedOrganization.identifier,
          item.view_id,
        )
        .then(async (res) => {
          if (res.status == 200) {
            store.dispatch("setSavedViewFlag", true);
            const extractedObj = res.data.data;

            // Resetting columns as its not required in searchObj
            // As we reassign columns from selectedFields and search results
            extractedObj.data.resultGrid.columns = [];

            // As in saved view, we observed field getting duplicated in selectedFields
            // So, we are removing duplicates before applying saved view
            if (extractedObj.data.stream.selectedFields?.length) {
              extractedObj.data.stream.selectedFields = [
                ...new Set(extractedObj.data.stream.selectedFields),
              ];
            }

            if (extractedObj.data?.timezone) {
              store.dispatch("setTimezone", extractedObj.data.timezone);
            }

            if (!extractedObj.data.stream.hasOwnProperty("streamType")) {
              extractedObj.data.stream.streamType = "logs";
            }

            delete searchObj.data.queryResults.aggs;

            if (
              searchObj.data.stream.streamType ==
              extractedObj.data.stream.streamType
            ) {
              // if (
              //   extractedObj.data.stream.selectedStream.value !=
              //   searchObj.data.stream.selectedStream.value
              // ) {
              //   extractedObj.data.stream.streamLists =
              //     searchObj.data.stream.streamLists;
              // }
              // ----- Here we are explicitly handling stream change for multistream -----
              let selectedStreams = [];
              const streamValues = searchObj.data.stream.streamLists.map(
                (item) => item.value,
              );
              if (typeof extractedObj.data.stream.selectedStream == "object") {
                if (
                  extractedObj.data.stream.selectedStream.hasOwnProperty(
                    "value",
                  )
                ) {
                  selectedStreams.push(
                    extractedObj.data.stream.selectedStream.value,
                  );
                } else {
                  selectedStreams.push(
                    ...extractedObj.data.stream.selectedStream,
                  );
                }
              } else {
                selectedStreams.push(extractedObj.data.stream.selectedStream);
              }
              const streamNotExist = selectedStreams.filter(
                (stream_str) => !streamValues.includes(stream_str),
              );
              if (streamNotExist.length > 0) {
                let errMsg = t("search.streamNotExist").replace(
                  "[STREAM_NAME]",
                  streamNotExist,
                );
                throw new Error(errMsg);
                return;
              }
              // extractedObj.data.stream.selectedStream = [];
              // extractedObj.data.stream.selectedStream = selectedStreams;
              delete extractedObj.data.stream.streamLists;
              delete extractedObj.data.stream.selectedStream;
              delete searchObj.data.stream.selectedStream;
              delete searchObj.meta.regions;
              if (extractedObj.meta.hasOwnProperty("regions")) {
                searchObj.meta["regions"] = extractedObj.meta.regions;
              } else {
                searchObj.meta["regions"] = [];
              }
              delete searchObj.data.queryResults.aggs;
              delete searchObj.data.stream.interestingFieldList;
              searchObj.data.stream.selectedStream = [];
              extractedObj.data.transforms = searchObj.data.transforms;
              extractedObj.data.stream.functions =
                searchObj.data.stream.functions;
              extractedObj.data.histogram = {
                xData: [],
                yData: [],
                chartParams: {},
              };
              extractedObj.data.savedViews = searchObj.data.savedViews;
              extractedObj.data.queryResults = [];
              extractedObj.meta.scrollInfo = {};
              searchObj.value = mergeDeep(searchObj, extractedObj);
              searchObj.shouldIgnoreWatcher = true;
              // await nextTick();
              if (extractedObj.data.tempFunctionContent != "") {
                populateFunctionImplementation(
                  {
                    name: "",
                    function: searchObj.data.tempFunctionContent,
                  },
                  false,
                );
                searchObj.data.tempFunctionContent =
                  extractedObj.data.tempFunctionContent;
                searchObj.meta.functionEditorPlaceholderFlag = false;
                searchObj.meta.toggleFunction = true;
              } else {
                populateFunctionImplementation(
                  {
                    name: "",
                    function: "",
                  },
                  false,
                );
                searchObj.data.tempFunctionContent = "";
                searchObj.meta.functionEditorPlaceholderFlag = true;
              }
              dateTimeRef.value.setSavedDate(searchObj.data.datetime);
              if (searchObj.meta.refreshInterval != "0") {
                onRefreshIntervalUpdate();
              } else {
                clearInterval(store.state.refreshIntervalID);
              }
              searchObj.data.stream.selectedStream.push(...selectedStreams);
              await updatedLocalLogFilterField();
              await getStreams("logs", true);
            } else {
              // ----- Here we are explicitly handling stream change -----
              resetStreamData();
              searchObj.data.stream.streamType =
                extractedObj.data.stream.streamType;

              delete searchObj.meta.regions;
              if (extractedObj.meta.hasOwnProperty("regions")) {
                searchObj.meta["regions"] = extractedObj.meta.regions;
              } else {
                searchObj.meta["regions"] = [];
              }
              // Here copying selected stream object, as in loadStreamLists() we are setting selected stream object to empty object
              // After loading stream list, we are setting selected stream object to copied object
              // const selectedStream = cloneDeep(
              //   extractedObj.data.stream.selectedStream
              // );
              let selectedStreams = [];
              if (typeof extractedObj.data.stream.selectedStream == "object") {
                if (
                  extractedObj.data.stream.selectedStream.hasOwnProperty(
                    "value",
                  )
                ) {
                  selectedStreams.push(
                    extractedObj.data.stream.selectedStream.value,
                  );
                } else {
                  selectedStreams.push(
                    ...extractedObj.data.stream.selectedStream,
                  );
                }
              } else {
                selectedStreams.push(extractedObj.data.stream.selectedStream);
              }

              extractedObj.data.transforms = searchObj.data.transforms;
              extractedObj.data.histogram = {
                xData: [],
                yData: [],
                chartParams: {},
              };
              extractedObj.data.savedViews = searchObj.data.savedViews;
              extractedObj.data.queryResults = [];
              extractedObj.meta.scrollInfo = {};
              delete searchObj.data.queryResults.aggs;

              searchObj.value = mergeDeep(searchObj, extractedObj);
              searchObj.data.streamResults = {};

              const streamData = await getStreams(
                searchObj.data.stream.streamType,
                true,
              );
              searchObj.data.streamResults = streamData;
              await loadStreamLists();
              searchObj.data.stream.selectedStream = [selectedStreams];
              // searchObj.value = mergeDeep(searchObj, extractedObj);

              const streamValues = searchObj.data.stream.streamLists.map(
                (item) => item.value,
              );
              const streamNotExist = selectedStreams.filter(
                (stream_str) => !streamValues.includes(stream_str),
              );
              if (streamNotExist.length > 0) {
                let errMsg = t("search.streamNotExist").replace(
                  "[STREAM_NAME]",
                  streamNotExist,
                );
                throw new Error(errMsg);
                return;
              }
              // await nextTick();
              if (extractedObj.data.tempFunctionContent != "") {
                populateFunctionImplementation(
                  {
                    name: "",
                    function: searchObj.data.tempFunctionContent,
                  },
                  false,
                );
                searchObj.data.tempFunctionContent =
                  extractedObj.data.tempFunctionContent;
                searchObj.meta.functionEditorPlaceholderFlag = false;
              } else {
                populateFunctionImplementation(
                  {
                    name: "",
                    function: "",
                  },
                  false,
                );
                searchObj.data.tempFunctionContent = "";
                searchObj.meta.functionEditorPlaceholderFlag = true;
              }
              dateTimeRef.value.setSavedDate(searchObj.data.datetime);
              if (searchObj.meta.refreshInterval != "0") {
                onRefreshIntervalUpdate();
              } else {
                clearInterval(store.state.refreshIntervalID);
              }
              await updatedLocalLogFilterField();
            }
            
            if(searchObj.meta.toggleFunction == false) {
              searchObj.config.fnSplitterModel = 99.5;
              resetFunctionContent();
            }

            $q.notify({
              message: `${item.view_name} view applied successfully.`,
              color: "positive",
              position: "bottom",
              timeout: 1000,
            });
            setTimeout(async () => {
              try {
                searchObj.loading = true;
                searchObj.meta.refreshHistogram = true;
                await extractFields();
                await getQueryData();
                store.dispatch("setSavedViewFlag", false);
                updateUrlQueryParams();
                searchObj.shouldIgnoreWatcher = false;
              } catch (e) {
                searchObj.shouldIgnoreWatcher = false;
                console.log(e);
              }
            }, 1000);

            if (
              extractedObj.data.resultGrid.colOrder &&
              extractedObj.data.resultGrid.colOrder.hasOwnProperty(
                searchObj.data.stream.selectedStream,
              )
            ) {
              searchObj.data.stream.selectedFields =
                extractedObj.data.resultGrid.colOrder[
                  searchObj.data.stream.selectedStream
                ];
            } else {
              searchObj.data.stream.selectedFields =
                extractedObj.data.stream.selectedFields;
            }

            if (
              extractedObj.data.resultGrid.colSizes &&
              extractedObj.data.resultGrid.colSizes.hasOwnProperty(
                searchObj.data.stream.selectedStream,
              )
            ) {
              searchObj.data.resultGrid.colSizes[
                searchObj.data.stream.selectedStream
              ] =
                extractedObj.data.resultGrid.colSizes[
                  searchObj.data.stream.selectedStream
                ];
            }

            // } else {
            //   searchObj.value = mergeDeep(searchObj, extractedObj);
            //   await nextTick();
            //   updatedLocalLogFilterField();
            //   handleRunQuery();
            // }
          } else {
            searchObj.shouldIgnoreWatcher = false;
            store.dispatch("setSavedViewFlag", false);
            $q.notify({
              message: err.message || `Error while applying saved view.`,
              color: "negative",
              position: "bottom",
              timeout: 3000,
            });
          }
        })
        .catch((err) => {
          searchObj.shouldIgnoreWatcher = false;
          store.dispatch("setSavedViewFlag", false);
          $q.notify({
            message: `Error while applying saved view.`,
            color: "negative",
            position: "bottom",
            timeout: 1000,
          });
          console.log(err);
        });
      // const extractedObj = JSON.parse(b64DecodeUnicode(item.data));
      // searchObj.value = mergeDeep(searchObj, extractedObj);
      // await nextTick();
      // updatedLocalLogFilterField();
      // handleRunQuery();
    };

    const handleSavedView = () => {
      if (isSavedViewAction.value == "create") {
        if (
          savedViewName.value == "" ||
          /^[A-Za-z0-9 \-\_]+$/.test(savedViewName.value) == false
        ) {
          $q.notify({
            message: `Please provide valid view name.`,
            color: "negative",
            position: "bottom",
            timeout: 1000,
          });
        } else {
          saveViewLoader.value = true;
          createSavedViews(savedViewName.value);
        }
      }
      //  else {
      //   if (savedViewSelectedName.value.view_id) {
      //     saveViewLoader.value = false;
      //     showSavedViewConfirmDialog(() => {
      //       saveViewLoader.value = true;
      //       updateSavedViews(
      //         savedViewSelectedName.value.view_id,
      //         savedViewSelectedName.value.view_name,
      //       );
      //     });
      //   } else {
      //     $q.notify({
      //       message: `Please select saved view to update.`,
      //       color: "negative",
      //       position: "bottom",
      //       timeout: 1000,
      //     });
      //   }
      // }
    };

    const deleteSavedViews = async () => {
      try {
        savedviewsService
          .delete(
            store.state.selectedOrganization.identifier,
            deleteViewID.value,
          )
          .then((res) => {
            if (res.status == 200) {
              $q.notify({
                message: `View deleted successfully.`,
                color: "positive",
                position: "bottom",
                timeout: 1000,
              });
              getSavedViews();
            } else {
              $q.notify({
                message: `Error while deleting saved view. ${res.data.error_detail}`,
                color: "negative",
                position: "bottom",
                timeout: 1000,
              });
            }
          })
          .catch((err) => {
            $q.notify({
              message: `Error while deleting saved view.`,
              color: "negative",
              position: "bottom",
              timeout: 1000,
            });
            console.log(err);
          });
      } catch (e: any) {
        console.log("Error while getting saved views", e);
      }
    };

    const getSearchObj = () => {
      try {
        delete searchObj.meta.scrollInfo;
        delete searchObj?.value;
        let savedSearchObj = toRaw(searchObj);
        savedSearchObj = JSON.parse(JSON.stringify(savedSearchObj));

        delete savedSearchObj.data.queryResults;
        delete savedSearchObj.data.histogram;
        delete savedSearchObj.data.sortedQueryResults;
        delete savedSearchObj.data.stream.streamLists;
        delete savedSearchObj.data.stream.functions;
        delete savedSearchObj.data.streamResults;
        delete savedSearchObj.data.savedViews;
        delete savedSearchObj.data.transforms;

        savedSearchObj.data.timezone = store.state.timezone;
        delete savedSearchObj.value;

        return savedSearchObj;
        // return b64EncodeUnicode(JSON.stringify(savedSearchObj));
      } catch (e) {
        console.log("Error while encoding search obj", e);
      }
    };

    const createSavedViews = (viewName: string) => {
      try {
        if (viewName.trim() == "") {
          $q.notify({
            message: `Please provide valid view name.`,
            color: "negative",
            position: "bottom",
            timeout: 1000,
          });
          saveViewLoader.value = false;
          return;
        }

        const viewObj: any = {
          data: getSearchObj(),
          view_name: viewName,
        };

        savedviewsService
          .post(store.state.selectedOrganization.identifier, viewObj)
          .then((res) => {
            if (res.status == 200) {
              store.dispatch("setSavedViewDialog", false);
              if (searchObj.data.hasOwnProperty("savedViews") == false) {
                searchObj.data.savedViews = [];
              }
              searchObj.data.savedViews.push({
                org_id: res.data.org_id,
                payload: viewObj.data,
                view_id: res.data.view_id,
                view_name: viewName,
              });
              $q.notify({
                message: `View created successfully.`,
                color: "positive",
                position: "bottom",
                timeout: 1000,
              });
              getSavedViews();
              isSavedViewAction.value = "create";
              savedViewName.value = "";
              saveViewLoader.value = false;
            } else {
              saveViewLoader.value = false;
              $q.notify({
                message: `Error while creating saved view. ${res.data.error_detail}`,
                color: "negative",
                position: "bottom",
                timeout: 1000,
              });
            }
          })
          .catch((err) => {
            saveViewLoader.value = false;
            $q.notify({
              message: `Error while creating saved view.`,
              color: "negative",
              position: "bottom",
              timeout: 1000,
            });
            console.log(err);
          });
      } catch (e: any) {
        isSavedViewAction.value = "create";
        savedViewName.value = "";
        saveViewLoader.value = false;
        $q.notify({
          message: `Error while saving view: ${e}`,
          color: "negative",
          position: "bottom",
          timeout: 1000,
        });
        console.log("Error while saving view", e);
      }
    };

    const updateSavedViews = (viewID: string, viewName: string) => {
      try {
        const viewObj: any = {
          data: getSearchObj(),
          view_name: viewName,
        };

        const dismiss = $q.notify({
          message: "Updating saved view...",
          position: "bottom",
          timeout: 0,
        });

        savedviewsService
          .put(store.state.selectedOrganization.identifier, viewID, viewObj)
          .then((res) => {
            dismiss();
            if (res.status == 200) {
              store.dispatch("setSavedViewDialog", false);
              //update the payload and view_name in savedViews object based on id
              searchObj.data.savedViews.forEach(
                (item: { view_id: string }, index: string | number) => {
                  if (item.view_id == viewID) {
                    searchObj.data.savedViews[index].payload = viewObj.data;
                    searchObj.data.savedViews[index].view_name = viewName;
                  }
                },
              );

              $q.notify({
                message: `View updated successfully.`,
                color: "positive",
                position: "bottom",
                timeout: 1000,
              });
              isSavedViewAction.value = "create";
              savedViewSelectedName.value = "";
              saveViewLoader.value = false;
              confirmSavedViewDialogVisible.value = false;
            } else {
              saveViewLoader.value = false;
              $q.notify({
                message: `Error while updating saved view. ${res.data.error_detail}`,
                color: "negative",
                position: "bottom",
                timeout: 1000,
              });
            }
          })
          .catch((err) => {
            dismiss();
            saveViewLoader.value = false;
            $q.notify({
              message: `Error while updating saved view.`,
              color: "negative",
              position: "bottom",
              timeout: 1000,
            });
            console.log(err);
          });
      } catch (e: any) {
        isSavedViewAction.value = "create";
        savedViewSelectedName.value = "";
        saveViewLoader.value = false;
        $q.notify({
          message: `Error while saving view: ${e}`,
          color: "negative",
          position: "bottom",
          timeout: 1000,
        });
        console.log("Error while saving view", e);
      }
    };

    const shareLink = useLoading(async () => {
      const queryObj = generateURLQuery(true);
      const queryString = Object.entries(queryObj)
        .map(
          ([key, value]) =>
            `${encodeURIComponent(key)}=${encodeURIComponent(value)}`,
        )
        .join("&");

      let shareURL = window.location.origin + window.location.pathname;

      if (queryString != "") {
        shareURL += "?" + queryString;
      }

      await shortURLService
        .create(store.state.selectedOrganization.identifier, shareURL)
        .then((res: any) => {
          if (res.status == 200) {
            shareURL = res.data.short_url;
            copyToClipboard(shareURL)
              .then(() => {
                $q.notify({
                  type: "positive",
                  message: "Link Copied Successfully!",
                  timeout: 5000,
                });
              })
              .catch(() => {
                $q.notify({
                  type: "negative",
                  message: "Error while copy link.",
                  timeout: 5000,
                });
              });
          }
        })
        .catch(() => {
          $q.notify({
            type: "negative",
            message: "Error while shortening link.",
            timeout: 5000,
          });
        });
    });
    const showSearchHistoryfn = () => {
      emit("showSearchHistory");
    };

    const QUERY_TEMPLATE = 'SELECT [FIELD_LIST] FROM "[STREAM_NAME]"';

    function getFieldList(
      stream,
      streamFields,
      interestingFields,
      isQuickMode,
    ) {
      searchObj.data.streamResults.list.forEach((item) => {
        if (
          item.name == stream &&
          Object.hasOwn(item, "schema") &&
          item.schema.length > 0
        ) {
          streamFields = item.schema;
        }
      });
      return streamFields
        .filter((item) => interestingFields.includes(item.name))
        .map((item) => item.name);
    }

    function buildStreamQuery(stream, fieldList, isQuickMode) {
      return QUERY_TEMPLATE.replace("[STREAM_NAME]", stream).replace(
        "[FIELD_LIST]",
        fieldList.length > 0 && isQuickMode ? fieldList.join(",") : "*",
      );
    }

    const resetFilters = () => {
      if (searchObj.meta.sqlMode == true) {
        const parsedSQL = fnParsedSQL();
        if (Object.hasOwn(parsedSQL, "from") && parsedSQL.from.length > 0) {
          if (Object.hasOwn(parsedSQL, "where") && parsedSQL.where != "") {
            parsedSQL.where = null;
          }

          if (Object.hasOwn(parsedSQL, "limit") && parsedSQL.limit != "") {
            parsedSQL.limit = null;
          }

          if (Object.hasOwn(parsedSQL, "_next") && parsedSQL._next != "") {
            parsedSQL._next.where = null;
            parsedSQL._next.limit = null;
          }

          searchObj.data.query = fnUnparsedSQL(parsedSQL);
          searchObj.data.query = searchObj.data.query.replaceAll("`", '"');
          searchObj.data.editorValue = searchObj.data.query;
        } else {
          // Handle both single and multiple stream scenarios
          const queries = searchObj.data.stream.selectedStream
            .map((stream) => {
              // Destructure for better readability
              const { selectedStreamFields, interestingFieldList } =
                searchObj.data.stream;
              const { quickMode } = searchObj.meta;

              // Generate the field list for the current stream
              const fieldList = getFieldList(
                stream,
                selectedStreamFields,
                interestingFieldList,
                quickMode,
              );

              // Ensure fieldList is valid before building the query
              if (!fieldList || fieldList.length === 0) {
                console.warn(`No fields available for stream: ${stream}`);
                return null;
              }

              // Build and return the query for the current stream
              return buildStreamQuery(stream, fieldList, quickMode);
            })
            .filter(Boolean);

          searchObj.data.query = queries.join(" UNION ");
          searchObj.data.editorValue = searchObj.data.query;
        }
      } else {
        searchObj.data.query = "";
        searchObj.data.editorValue = "";
      }

      queryEditorRef.value?.setValue(searchObj.data.query);
      if (store.state.zoConfig.query_on_stream_selection == false) {
        handleRunQueryFn();
      }
    };

    const customDownloadDialog = ref(false);
    const downloadCustomInitialNumber = ref(1);
    const downloadCustomRange = ref(100);
    const downloadCustomRangeOptions = ref([100, 500, 1000, 5000, 10000]);

    const loadSavedView = () => {
      if (searchObj.data.savedViews.length == 0) {
        getSavedViews();
      }
    };

    const handleFavoriteSavedView = (row: any, flag: boolean) => {
      let localSavedView: any = {};
      let savedViews = useLocalSavedView();

      if (savedViews.value != null) {
        localSavedView = savedViews.value;
      }

      Object.keys(localSavedView).forEach((item, key) => {
        if (item == row.view_id) {
          if (flag) {
            delete localSavedView[item];
            useLocalSavedView(localSavedView);
            const index = favoriteViews.value.indexOf(row.view_id);
            if (index > -1) {
              favoriteViews.value.splice(index, 1);
            }

            let favoriteViewsList = localSavedViews.value;
            if (favoriteViewsList.length > 0) {
              favoriteViewsList = favoriteViewsList.filter(
                (item) => item.view_id != row.view_id,
              );
              // for (const [key, item] of favoriteViewsList.entries()) {
              //   console.log(item, key);
              //   if (item.view_id == row.view_id) {
              //     delete favoriteViewsList[key];
              //   }
              // }
              console.log(favoriteViewsList);
              localSavedViews.value = favoriteViewsList;
            }
          }
        }
      });

      if (!flag) {
        if (favoriteViews.value.length >= 10) {
          $q.notify({
            message: "You can only save 10 views.",
            color: "info",
            position: "bottom",
            timeout: 2000,
          });
          return;
        }
        localSavedView[row.view_id] = JSON.parse(JSON.stringify(row));
        favoriteViews.value.push(row.view_id);
        localSavedViews.value.push(row);

        // moveItemsToTop(localSavedView, favoriteViews.value);

        useLocalSavedView(localSavedView);
        $q.notify({
          message: "View added to favorites.",
          color: "positive",
          position: "bottom",
          timeout: 2000,
        });
      } else {
        // alert(favoriteViews.value.length)
        // moveItemsToTop(localSavedView, favoriteViews.value);
        $q.notify({
          message: "View removed from favorites.",
          color: "positive",
          position: "bottom",
          timeout: 2000,
        });
      }
    };

    const filterSavedViewFn = (rows: any, terms: any) => {
      var filtered = [];
      if (terms != "") {
        terms = terms.toLowerCase();
        for (var i = 0; i < rows.length; i++) {
          if (rows[i]["view_name"].toLowerCase().includes(terms)) {
            filtered.push(rows[i]);
          }
        }
      }
      return filtered;
    };

    const regionFilterMethod = (node, filter) => {
      const filt = filter.toLowerCase();
      return node.label && node.label.toLowerCase().indexOf(filt) > -1;
    };
    const resetRegionFilter = () => {
      regionFilter.value = "";
    };

    const handleRegionsSelection = (item, isSelected) => {
      if (isSelected) {
        const index = searchObj.meta.regions.indexOf(item);
        if (index > -1) {
          searchObj.meta.regions.splice(index, 1);
        }
      } else {
        searchObj.meta.regions.push(item);
      }
    };

    const handleQuickMode = () => {
      emit("handleQuickModeChange");
    };

    const handleRunQueryFn = () => {
      if (searchObj.meta.logsVisualizeToggle == "visualize") {
        emit("handleRunQueryFn");
      } else {
        handleRunQuery();
      }
    };

    const onLogsVisualizeToggleUpdate = (value: any) => {
      // confirm with user on toggle from visualize to logs
      if (
        value == "logs" &&
        searchObj.meta.logsVisualizeToggle == "visualize"
      ) {
        confirmLogsVisualizeModeChangeDialog.value = true;
      } else {
        searchObj.meta.logsVisualizeToggle = value;
      }
    };

    const dashboardPanelDataPageKey = inject(
      "dashboardPanelDataPageKey",
      "logs",
    );
    const { dashboardPanelData, resetDashboardPanelData } =
      useDashboardPanelData(dashboardPanelDataPageKey);

    const isVisualizeToggleDisabled = computed(() => {
      return searchObj.data.stream.selectedStream.length > 1;
    });

    const changeLogsVisualizeToggle = () => {
      // change logs visualize toggle
      searchObj.meta.logsVisualizeToggle = "logs";
      confirmLogsVisualizeModeChangeDialog.value = false;

      // store dashboardPanelData meta object
      const dashboardPanelDataMetaObj = dashboardPanelData.meta;

      // reset old dashboardPanelData
      resetDashboardPanelData();

      // assign, old dashboardPanelData meta object
      dashboardPanelData.meta = dashboardPanelDataMetaObj;
    };

    // [START] cancel running queries

    const variablesAndPanelsDataLoadingState =
      inject("variablesAndPanelsDataLoadingState", {}) || {};

    const visualizeSearchRequestTraceIds = computed(() => {
      const searchIds = Object.values(
        variablesAndPanelsDataLoadingState?.searchRequestTraceIds,
      ).filter((item: any) => item.length > 0);

      return searchIds.flat() as string[];
    });
    const backgroundColorStyle = computed(() => {
      const isDarkMode = store.state.theme === "dark";
      return {
        backgroundColor:
          searchObj.meta.toggleFunction && isFocused.value
            ? isDarkMode
              ? "#575A5A"
              : "#E0E0E0" // Dark mode: grey, Light mode: yellow (or any color)
            : "",
        borderBottom:
          searchObj.meta.toggleFunction && isFocused.value
            ? isDarkMode
              ? "2px solid #575A5A "
              : "2px solid #E0E0E0"
            : "none",
      };
    });
    const editorWidthToggleFunction = computed(() => {
      const isDarkMode = store.state.theme === "dark";

      if (!searchObj.meta.toggleFunction && isFocused.value) {
        return {
          width: `calc(100 - ${searchObj.config.fnSplitterModel})%`,
          borderBottom: isDarkMode ? "2px solid #575A5A" : "2px solid #E0E0E0",
        };
      } else {
        return {
          width: "100%",
          borderBottom: "none",
        };
      }
    });
    const { traceIdRef, cancelQuery: cancelVisualizeQuery } = useCancelQuery();

    const cancelVisualizeQueries = () => {
      traceIdRef.value = visualizeSearchRequestTraceIds.value;
      cancelVisualizeQuery();
    };

    const disable = ref(false);

    watch(variablesAndPanelsDataLoadingState, () => {
      const panelsValues = Object.values(
        variablesAndPanelsDataLoadingState?.panels,
      );
      disable.value = panelsValues.some((item: any) => item === true);
    });
    const iconRight = computed(() => {
      return (
        "img:" +
        getImageURL(
          store.state.theme === "dark"
            ? "images/common/function_dark.svg"
            : "images/common/function.svg",
        )
      );
    });
    const functionToggleIcon = computed(() => {
      return (
        "img:" +
        getImageURL(
          searchObj.meta.toggleFunction
            ? "images/common/function_dark.svg"
            : "images/common/function.svg",
        )
      );
    });
    const addJobScheduler = async () => {

      try{
        // if(searchObj.meta.jobId != ""){
        //   searchObj.meta.jobId = "";
        // }
        if (searchObj.meta.jobId != "") {
          $q.notify({
            type: "negative",
            message: "Job Already Scheduled , please change some parameters to schedule new job",
            timeout: 3000,
          });
          return;
        }
        if (
          searchObj.meta.jobRecords > 100000 ||
          searchObj.meta.jobRecords == 0 ||
          searchObj.meta.jobRecords < 0
        ) {
          $q.notify({
            type: "negative",
            message: "Job Scheduler should be between 1 and 100000",
            timeout: 3000,
          });
          return;
        }

        searchSchedulerJob.value = false;
        searchObj.meta.showSearchScheduler = false;
        await getJobData();

      }
      catch (e){
        $q.notify({
          type: "negative",
          message: "Error while adding job",
          timeout: 3000,
        });
        return;
      }

    };

    const createScheduleJob = () => {
      searchSchedulerJob.value = true;
      searchObj.meta.jobRecords = 100;
    }

    const checkQuery = (query) => {
      const jobQuery = router.currentRoute.value.query.query;
      if (jobQuery == b64EncodeUnicode(query)) {
        return true;
      }
      return false;
    }
    const checkFnQuery = (fnQuery) => {
      const jobFnQuery = router.currentRoute.value.query.functionContent;
      if (jobFnQuery == b64EncodeUnicode(fnQuery)) {
        return true;
      }
      return false;
    }

    // [END] cancel running queries

    return {
      $q,
      t,
      store,
      router,
      fnEditorRef,
      searchObj,
      queryEditorRef,
      confirmDialogVisible,
      confirmCallback,
      refreshTimes: searchObj.config.refreshTimes,
      refreshTimeChange,
      updateQueryValue,
      updateDateTime,
      showConfirmDialog,
      showSavedViewConfirmDialog,
      cancelConfirmDialog,
      confirmDialogOK,
      updateQuery,
      downloadLogs,
      saveFunction,
      resetFunctionContent,
      resetEditorLayout,
      populateFunctionImplementation,
      functionModel,
      functionOptions,
      filterFn,
      refreshData,
      handleRunQuery,
      handleRunQueryFn,
      autoCompleteKeywords,
      autoCompleteSuggestions,
      onRefreshIntervalUpdate,
      updateTimezone,
      dateTimeRef,
      fnSavedView,
      applySavedView,
      isSavedViewAction,
      savedViewName,
      savedViewSelectedName,
      handleSavedView,
      deleteSavedViews,
      deleteViewID,
      confirmDelete,
      saveViewLoader,
      savedViewDropdownModel,
      fnSavedFunctionDialog,
      isSavedFunctionAction,
      savedFunctionName,
      savedFunctionSelectedName,
      saveFunctionLoader,
      shareLink,
      showSearchHistoryfn,
      getImageURL,
      resetFilters,
      customDownloadDialog,
      downloadCustomInitialNumber,
      downloadCustomRange,
      downloadCustomRangeOptions,
      buildSearch,
      confirmSavedViewDialogVisible,
      rowsPerPage,
      handleFavoriteSavedView,
      favoriteViews,
      localSavedViews,
      loadSavedView,
      filterSavedViewFn,
      config,
      handleRegionsSelection,
      handleQuickMode,
      regionFilterMethod,
      regionFilterRef,
      regionFilter,
      resetRegionFilter,
      validateFilterForMultiStream,
      cancelQuery,
      confirmLogsVisualizeModeChangeDialog,
      changeLogsVisualizeToggle,
      isVisualizeToggleDisabled,
      onLogsVisualizeToggleUpdate,
      visualizeSearchRequestTraceIds,
      disable,
      cancelVisualizeQueries,
      isFocused,
      backgroundColorStyle,
      editorWidthToggleFunction,
      fnParsedSQL,
      iconRight,
      functionToggleIcon,
      searchSchedulerJob,
      autoSearchSchedulerJob,
      addJobScheduler,
      routeToSearchSchedule,
      createScheduleJob,
      searchTerm,
      filteredFunctionOptions,
      confirmUpdate,
      updateViewObj,
      updateSavedViews,
      checkQuery,
      checkFnQuery,
    };
  },
  computed: {
    addSearchTerm() {
      return this.searchObj.data.stream.addToFilter;
    },
    toggleFunction() {
      return this.searchObj.meta.toggleFunction;
    },
    confirmMessage() {
      return "Are you sure you want to update the function?";
    },
    confirmMessageSavedView() {
      return "Are you sure you want to update the saved view?";
    },
    resetFunction() {
      return this.searchObj.data.tempFunctionName;
    },
    resetFunctionDefinition() {
      return this.searchObj.data.tempFunctionContent;
    },
  },
  watch: {
    addSearchTerm() {
      if (this.searchObj.data.stream.addToFilter != "") {
        let currentQuery = this.searchObj.data.query.split("|");
        if (currentQuery.length > 1) {
          if (currentQuery[1].trim() != "") {
            currentQuery[1] += " and " + filter;
          } else {
            currentQuery[1] = filter;
          }
          this.searchObj.data.query = currentQuery.join("| ");
          this.searchObj.data.editorValue = this.searchObj.data.query;
        } else {
          let unionType: string = "";
          if (
            currentQuery[0]
              .replace("union all", "UNION ALL")
              .includes("UNION ALL")
          ) {
            unionType = "UNION ALL";
          } else if (
            currentQuery[0].replace("union", "UNION").includes("UNION")
          ) {
            unionType = "UNION";
          }

          // Use regular expression to match "UNION" or "UNION ALL" (case insensitive)
          const unionRegex = /\bUNION ALL\b|\bUNION\b/i;

          // Split the string by "UNION" or "UNION ALL" if they are present
          const queries = currentQuery[0].split(unionRegex);

          // Iterate over each part
          queries.forEach((query, index) => {
            let filter = this.searchObj.data.stream.addToFilter;

            const isFilterValueNull = filter.split(/=|!=/)[1] === "'null'";

            if (isFilterValueNull) {
              filter = filter
                .replace(/=|!=/, (match) => {
                  return match === "=" ? " is " : " is not ";
                })
                .replace(/'null'/, "null");
            }

            if (this.searchObj.meta.sqlMode == true) {
              if (
                unionType == "" &&
                this.searchObj.data.stream.selectedStream.length > 1
              ) {
                const parsedSQL = this.fnParsedSQL();
                const streamPrefix: string =
                  parsedSQL.from[0].as != null
                    ? parsedSQL.from[0].as
                    : parsedSQL.from[0].table;
                filter = `"${streamPrefix}".${filter}`;
              }

              // if query contains order by clause or limit clause then add where clause before that
              // if query contains where clause then add filter after that with and operator and keep order by or limit after that
              // if query does not contain where clause then add where clause before filter
              if (query.toLowerCase().includes("where")) {
                if (query.toLowerCase().includes("order by")) {
                  const [beforeOrderBy, afterOrderBy] = queryIndexSplit(
                    query,
                    "order by",
                  );
                  query =
                    beforeOrderBy.trim() +
                    " AND " +
                    filter +
                    " order by" +
                    afterOrderBy;
                } else if (query.toLowerCase().includes("limit")) {
                  const [beforeLimit, afterLimit] = queryIndexSplit(
                    query,
                    "limit",
                  );
                  query =
                    beforeLimit.trim() +
                    " AND " +
                    filter +
                    " limit" +
                    afterLimit;
                } else {
                  query = query + " AND " + filter;
                }
              } else {
                if (query.toLowerCase().includes("order by")) {
                  const [beforeOrderBy, afterOrderBy] = queryIndexSplit(
                    query,
                    "order by",
                  );
                  query =
                    beforeOrderBy.trim() +
                    " where " +
                    filter +
                    " order by" +
                    afterOrderBy;
                } else if (query.toLowerCase().includes("limit")) {
                  const [beforeLimit, afterLimit] = queryIndexSplit(
                    query,
                    "limit",
                  );
                  query =
                    beforeLimit.trim() +
                    " where " +
                    filter +
                    " limit" +
                    afterLimit;
                } else {
                  query = query + " where " + filter;
                }
              }
              currentQuery[0] = query;
            } else {
              currentQuery[0].length == 0
                ? (currentQuery[0] = filter)
                : (currentQuery[0] += " and " + filter);
            }

            // this.searchObj.data.query = currentQuery[0];
            queries[index] = currentQuery[0];
          });

          if (unionType == "") {
            this.searchObj.data.query = queries.join("");
          } else {
            this.searchObj.data.query = queries.join(` ${unionType} `);
          }
          this.searchObj.data.editorValue = this.searchObj.data.query;
          this.searchObj.data.stream.addToFilter = "";
          if (this.queryEditorRef?.setValue)
            this.queryEditorRef.setValue(this.searchObj.data.query);
        }
      }
    },
    toggleFunction(newVal) {
      if (newVal == false) {
        this.searchObj.config.fnSplitterModel = 99.5;
        this.resetFunctionContent();
      } else {
        this.searchObj.config.fnSplitterModel = 60;
      }
      this.resetEditorLayout();
    },
    resetFunction(newVal) {
      if (newVal == "" && store.state.savedViewFlag == false) {
        this.resetFunctionContent();
      }
    },
    resetFunctionDefinition(newVal) {
      if (newVal == "") this.resetFunctionContent();
    },
  },
});
</script>

<style lang="scss">
.logs-saved-view-icon:hover {
  color: black !important;
  background-color: lightgray !important;
}
.logs-search-bar-component {
  padding-bottom: 1px;
  height: 100%;
  overflow: visible;

  .reset-filters {
    width: 32px;
    height: 32px;

    .q-icon {
      margin-right: 0;
    }
  }

  #logsQueryEditor,
  #fnEditor {
    height: 100% !important;
  }
  #fnEditor {
    width: 100%;
    border-radius: 5px;
    border: 0px solid #dbdbdb;
    overflow: hidden;
  }

  .q-field--standard .q-field__control:before,
  .q-field--standard .q-field__control:focus:before,
  .q-field--standard .q-field__control:hover:before {
    border: 0px !important;
    border-color: none;
    transition: none;
  }

  .row:nth-child(2) {
    height: 100%; /* or any other height you want to set */
  }

  .empty-query .monaco-editor-background {
    background-image: url("../../assets/images/common/query-editor.png");
    background-repeat: no-repeat;
    background-size: 115px;
  }

  .empty-function .monaco-editor-background {
    background-image: url("../../assets/images/common/vrl-function.png");
    background-repeat: no-repeat;
    background-size: 170px;
  }

  .function-dropdown {
    width: 205px;
    padding-bottom: 0px;
    border: 1px solid #dbdbdb;
    border-radius: 5px;
    cursor: pointer;

    .q-field__input {
      cursor: pointer;
      font-weight: 600;
      font-size: 12px;
    }
    .q-field__native,
    .q-field__control {
      min-height: 29px;
      height: 29px;
      padding: 0px 0px 0px 4px;
    }

    .q-field__marginal {
      height: 30px;
    }
  }

  .q-toggle__inner {
    font-size: 30px;
  }

  .q-toggle__label {
    font-size: 12px;
  }

  .casesensitive-btn {
    padding: 8px;
    margin-left: -6px;
    background-color: #d5d5d5;
    border-radius: 0px 3px 3px 0px;
  }

  .search-field .q-field {
    &__control {
      border-radius: 3px 0px 0px 3px !important;
    }

    &__native {
      font-weight: 600;
    }
  }

  .search-time {
    // width: 120px;
    .q-btn-group {
      border-radius: 3px;

      .q-btn {
        min-height: auto;
      }
    }
  }

  .search-dropdown {
    padding: 0px;

    .block {
      color: $dark-page;
      font-weight: 600;
      font-size: 12px;
    }

    .q-btn-dropdown__arrow-container {
      color: $light-text2;
    }
  }

  .refresh-rate-dropdown-container {
    width: 220px;

    * .q-btn {
      font-size: 12px !important;
      padding-left: 8px;
      padding-right: 8px;
    }
  }

  .flex-start {
    justify-content: flex-start;
    align-items: flex-start;
    display: flex;
  }

  .resultsOverChart {
    margin-bottom: 0.75rem;
    font-size: 0.875rem;
    color: $dark-page;
    font-weight: 700;
  }

  .ddlWrapper {
    position: relative;
    z-index: 10;

    .listWrapper {
      box-shadow: 0px 3px 15px rgba(0, 0, 0, 0.1);
      transition: height 0.25s ease;
      height: calc(100vh - 146px);
      background-color: white;
      position: absolute;
      top: 2.75rem;
      width: 100%;
      left: 0;

      &:empty {
        height: 0;
      }

      &,
      .q-list {
        border-radius: 3px;
      }
    }
  }

  .fields_autocomplete {
    max-height: 250px;
  }

  .search-button {
    min-width: 77px;
    line-height: 29px;
    font-weight: bold;
    text-transform: initial;
    font-size: 11px;
    color: white;

    .q-btn__content {
      background: $secondary;
      border-radius: 3px 3px 3px 3px;
      padding: 0px 5px;

      .q-icon {
        font-size: 15px;
        color: #ffffff;
      }
    }
  }

  .cancel-search-button {
    .q-btn__content {
      background: $negative !important;
    }
  }

  .download-logs-btn {
    height: 30px;
  }

  .query-editor-container {
    height: calc(100% - 35px) !important;
  }

  .logs-auto-refresh-interval {
    .q-btn {
      min-height: 30px;
      max-height: 30px;
      padding: 0 4px;
    }
  }

  .saved-views-dropdown {
    border-radius: 4px;
    button {
      padding: 4px 5px;
    }
  }

  .savedview-dropdown {
    width: 215px;
    display: inline-block;
    border: 1px solid #dbdbdb;

    .q-field__input {
      cursor: pointer;
      font-weight: 600;
      font-size: 12px;
    }
    .q-field__native,
    .q-field__control {
      min-height: 29px !important;
      height: 29px;
      padding: 0px 0px 0px 4px;
    }

    .q-field__marginal {
      height: 30px;
    }
  }

  .saved-view-item {
    padding: 2px 4px !important;
  }

  .body--dark {
    .btn-function {
      filter: brightness(100);
    }
  }

  .q-pagination__middle > .q-btn {
    min-width: 30px !important;
    max-width: 30px !important;
  }

  .q-item {
    padding: 0px !important;
  }

  .q-focus-helper:hover {
    background: transparent !important;
  }

  .favorite-label {
    line-height: 24px !important;
    font-weight: bold !important;
  }

  .region-dropdown-btn {
    text-transform: capitalize;
    font-weight: 600;
    font-size: 12px;
    padding-left: 8px;
    height: 30px;
    padding-top: 3px;

    .q-btn-dropdown__arrow {
      margin-left: 0px !important;
    }
  }

  .download-logs-btn {
    .q-btn-dropdown__arrow {
      margin-left: 0px !important;
    }
  }

  .region-dropdown-list {
    min-width: 150px;

    .q-item__section {
      display: inline-block;
    }

    .q-item__label {
      margin-left: 20px;
      text-transform: capitalize;
      margin-top: 2px;
    }
  }
}
.saved-view-table {
  td {
    padding: 0;
    height: 25px !important;
    min-height: 25px !important;
  }

  .q-table__control {
    margin: 0px !important;
    width: 100% !important;
    text-align: right;
  }

  .q-table__bottom {
    padding: 0px !important;
    min-height: 35px;

    .q-table__control {
      padding: 0px 10px !important;
    }
  }

  .q-table__top {
    padding: 0px !important;
    margin: 0px !important;
    left: 0px;
    width: 100%;

    .q-table__separator {
      display: none;
    }

    .q-table__control {
      padding: 0px !important;
    }
  }

  .q-field--filled .q-field__control {
    padding: 0px 5px !important;
  }

  .saved-view-item {
    padding: 4px 5px 4px 10px !important;
  }

  .q-item__section--main ~ .q-item__section--side {
    padding-left: 5px !important;
  }
}

.logs-visualize-toggle {
  .button-group {
    border: 1px solid gray !important;
    border-radius: 9px;
  }

  .button {
    display: block;
    cursor: pointer;
    background-color: #f0eaea;
    border: none;
    font-size: 12px;
    padding: 6px 4px;
  }

  .button-left {
    border-top-left-radius: 4px;
    border-bottom-left-radius: 4px;
    color: black;
  }

  .button-right {
    border-top-right-radius: 4px;
    border-bottom-right-radius: 4px;
    color: black;
  }
  .selected {
    background-color: var(--q-primary) !important;
    color: white;
  }
}
</style>
<style scoped>
.expand-on-focus {
  height: calc(100vh - 200px) !important;
  z-index: 20 !important;
}
</style>
