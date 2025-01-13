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
  <!-- 
    Create a table like SearchResults.vue component.
    1. Props for columns, rows and loading
    2. use virtual scroll
    3. Column should have boolean property to close column. closable: true
    4. Column should have boolean property to hide column. hidden: true
    5. Component should have highlight property to highlight search text
    6. Rows should have boolean property to expand row. expandable: true
   -->
  <div :style="{ height: height, marginTop: 0 }" class="app-table-container">
    <q-table
      flat
      :bordered="bordered"
      ref="qTableRef"
      :title="title"
      :rows="rows"
      :columns="columns as []"
      :table-colspan="9"
      row-key="index"
      :virtual-scroll="virtualScroll"
      :virtual-scroll-item-size="48"
      :rows-per-page-options="[0]"
      :pagination="_pagination"
      :hide-header="hideHeader"
      :filter="filter && filter.value"
      :filter-method="filter && filter.method"
      @virtual-scroll="onScroll"
      class="full-height"
    >
      <template #no-data>
        <NoData class="q-mb-lg" />
      </template>
      <template v-slot:header="props">
        <q-tr :props="props" class="thead-sticky">
          <q-th
            v-for="col in props.cols"
            :key="col.name"
            :props="props"
            :style="col.style"
          >
            {{ col.label }}
          </q-th>
        </q-tr>
      </template>
       <template #top="scope">
        <QTablePagination
          :scope="scope"
          :resultTotal="resultTotal"
          :perPageOptions="perPageOptions"
          position="top"
          @update:changeRecordPerPage="changePagination"
        />
       </template>
      <template v-slot:body="props">
        <q-tr :props="props" :key="`m_${props.row.index}`">
          <q-td
            v-for="col in props.cols"
            :class="col.class || ''"
            :key="col.name"
            :props="props"
            @click="handleDataClick(col.name, props.row)"
          >
            <template v-if="col.slot">
              <slot
                :name="col.slotName"
                :column="{ ...props }"
                :columnName="col.name"
              />
            </template>
            <template v-else-if="col.type === 'action'">
              <q-icon :name="col.icon" size="24px" class="cursor-pointer" />
            </template>
            <template v-else>
              {{ col.value }}
            </template>
          </q-td>
        </q-tr>
        <q-tr
          v-show="props.row.expand"
          :props="props"
          :key="`e_${props.row.index + 'entity'}`"
          class="q-virtual-scroll--with-prev"
          style="transition: display 2s ease-in"
        >
          <q-td colspan="100%" style="padding: 0; border-bottom: none">
            <slot :name="props.row.slotName" :row="props" />
          </q-td>
        </q-tr>
      </template>
      <template  #bottom="scope">
        <QTablePagination
          :scope="scope"
          :position="'bottom'"
          :resultTotal="resultTotal"
          :perPageOptions="perPageOptions"
          @update:changeRecordPerPage="changePagination"
        />
      </template>
    </q-table>
  </div>
</template>

<script setup lang="ts">
import type { QTable } from "quasar";
import type { Ref } from "vue";
import { ref } from "vue";
import QTablePagination from "@/components/shared/grid/Pagination.vue";
import { computed,watch  } from "vue";
import NoData from "./shared/grid/NoData.vue";

const props = defineProps({
  columns: {
    type: Array,
    required: true,
  },
  rows: {
    type: Array,
    required: true,
  },
  loading: {
    type: Boolean,
    required: false,
    default: false,
  },
  highlight: {
    type: Boolean,
    required: false,
    default: false,
  },
  highlightText: {
    type: String,
    default: "",
  },
  title: {
    type: String,
    default: "",
  },
  virtualScroll: {
    type: Boolean,
    default: true,
  },
  height: {
    type: String,
    default: "100%",
  },
  pagination: {
    type: Boolean,
    default: false,
  },
  rowsPerPage: {
    type: Number,
    default: 25,
  },
  dense: {
    type: Boolean,
    default: false,
  },
  hideHeader: {
    type: Boolean,
    default: false,
  },
  filter: {
    type: Object,
    default: () => null,
  },
  bordered: {
    type: Boolean,
    default: true,
  },
});

const emit = defineEmits(["event-emitted"]);

const perPageOptions: any = [
      { label: "25", value: 25 },
      { label: "50", value: 50 },
      { label: "100", value: 100 },
      { label: "250", value: 250 },
      { label: "500", value: 500 }
];

const resultTotal = ref<number>(0);
const selectedPerPage = ref<number>(25);

const qTableRef: Ref<InstanceType<typeof QTable> | null> = ref(null);

const showPagination = computed(() => {
  return props.pagination;
});

const _pagination: any = ref({
  rowsPerPage: props.rowsPerPage,
});
const changePagination = (val: { label: string; value: any }) => {
  selectedPerPage.value = val.value;
  _pagination.value.rowsPerPage = val.value;
  qTableRef.value?.setPagination(_pagination.value);
};
const handleDataClick = (columnName: string, row: any) => {
  emit("event-emitted", "cell-click", { columnName, row });
};

const onScroll = (e: any) => {
  try {
    emit("event-emitted", "scroll", e);
  } catch (e) {
    console.log("error", e);
  }
};
watch(
  () => props.rows,
  (newRows) => {
    resultTotal.value = newRows.length;
  },
  { immediate: true } // Ensures it runs on initial load
);

</script>

<style lang="scss">
.app-table-container {
  .thead-sticky,
  .tfoot-sticky {
    position: sticky;
    top: 0;
    opacity: 1;
    z-index: 1;
    background: #f5f5f5;
  }

  .q-table--dark .thead-sticky,
  .q-table--dark .tfoot-sticky {
    background: #565656 !important;
  }

  .q-table__bottom {
    .q-table__control {
      padding-top: 0;
      padding-bottom: 0;
    }
  }
}
</style>
