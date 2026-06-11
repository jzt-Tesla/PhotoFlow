<script setup lang="ts">
import { ref } from "vue";
import { createTag, deleteTag, renameTag, updateTagColor, listTags } from "../api";
import type { Tag } from "../types";

const props = defineProps<{ tags: Tag[] }>();
const emit = defineEmits<{
  close: [];
  "tags-changed": [tags: Tag[]];
}>();

const newName = ref("");
const newColor = ref("#667eea");
const editingId = ref<number | null>(null);
const editName = ref("");
const error = ref("");

const colors = [
  "#667eea", "#764ba2", "#e57373", "#81c784",
  "#ffb74d", "#4fc3f7", "#f06292", "#a1887f",
];

async function addTag() {
  const name = newName.value.trim();
  if (!name) return;
  error.value = "";
  try {
    await createTag(name, newColor.value);
    newName.value = "";
    const tags = await listTags();
    emit("tags-changed", tags);
  } catch {
    error.value = "标签名已存在";
  }
}

async function removeTag(id: number) {
  await deleteTag(id);
  const tags = await listTags();
  emit("tags-changed", tags);
}

function startEdit(tag: Tag) {
  editingId.value = tag.id;
  editName.value = tag.name;
}

async function saveEdit() {
  if (editingId.value === null) return;
  const name = editName.value.trim();
  if (!name) return;
  try {
    await renameTag(editingId.value, name);
    editingId.value = null;
    const tags = await listTags();
    emit("tags-changed", tags);
  } catch {
    error.value = "标签名已存在";
  }
}

async function changeColor(tagId: number, color: string) {
  await updateTagColor(tagId, color);
  const tags = await listTags();
  emit("tags-changed", tags);
}
</script>

<template>
  <div class="dialog-overlay" @click.self="emit('close')">
    <div class="dialog">
      <div class="dialog-header">
        <h3>管理标签</h3>
        <button class="btn-close" @click="emit('close')">×</button>
      </div>

      <!-- Add tag -->
      <div class="add-row">
        <input
          v-model="newName"
          class="tag-input"
          placeholder="新标签名…"
          @keyup.enter="addTag"
        />
        <div class="color-palette">
          <button
            v-for="c in colors"
            :key="c"
            class="color-dot"
            :class="{ selected: newColor === c }"
            :style="{ background: c }"
            @click="newColor = c"
          />
        </div>
        <button class="btn-add" @click="addTag">添加</button>
      </div>
      <div v-if="error" class="error">{{ error }}</div>

      <!-- Tag list -->
      <div class="tag-list">
        <div v-for="tag in props.tags" :key="tag.id" class="tag-row">
          <template v-if="editingId === tag.id">
            <input v-model="editName" class="tag-input" @keyup.enter="saveEdit" @blur="saveEdit" />
          </template>
          <template v-else>
            <span class="tag-name" @dblclick="startEdit(tag)">
              <span class="tag-dot" :style="{ background: tag.color }" />
              {{ tag.name }}
            </span>
          </template>
          <div class="tag-actions">
            <div class="mini-palette">
              <button
                v-for="c in colors"
                :key="c"
                class="color-dot-sm"
                :style="{ background: c }"
                @click="changeColor(tag.id, c)"
              />
            </div>
            <button class="btn-delete" @click="removeTag(tag.id)">删除</button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10000;
}

.dialog {
  background: #16213e;
  border-radius: 12px;
  padding: 24px;
  width: 400px;
  max-height: 80vh;
  overflow-y: auto;
}

.dialog-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.dialog-header h3 {
  color: #e0e0e0;
  font-size: 1rem;
}

.btn-close {
  background: none;
  border: none;
  color: #888;
  font-size: 1.5rem;
  cursor: pointer;
}

.btn-close:hover {
  color: #e57373;
}

.add-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
  flex-wrap: wrap;
}

.tag-input {
  flex: 1;
  background: #1a1a2e;
  border: 1px solid #333;
  border-radius: 4px;
  color: #e0e0e0;
  padding: 6px 10px;
  font-size: 0.85rem;
  outline: none;
}

.tag-input:focus {
  border-color: #667eea;
}

.color-palette {
  display: flex;
  gap: 4px;
}

.color-dot {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
}

.color-dot.selected {
  border-color: #fff;
}

.color-dot-sm {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  border: 1px solid transparent;
  cursor: pointer;
}

.color-dot-sm:hover {
  border-color: #fff;
}

.btn-add {
  background: #667eea;
  border: none;
  border-radius: 4px;
  color: #fff;
  padding: 6px 12px;
  font-size: 0.8rem;
  cursor: pointer;
}

.btn-add:hover {
  opacity: 0.85;
}

.error {
  color: #e57373;
  font-size: 0.8rem;
  margin-bottom: 8px;
}

.tag-list {
  margin-top: 12px;
}

.tag-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 4px;
  border-bottom: 1px solid #1a1a2e;
}

.tag-name {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #bbb;
  font-size: 0.85rem;
  cursor: pointer;
}

.tag-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.tag-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.mini-palette {
  display: flex;
  gap: 2px;
}

.btn-delete {
  background: none;
  border: none;
  color: #e57373;
  font-size: 0.75rem;
  cursor: pointer;
}

.btn-delete:hover {
  text-decoration: underline;
}
</style>
