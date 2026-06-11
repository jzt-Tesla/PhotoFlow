<script setup lang="ts">
import PhotoGallery from "./PhotoGallery.vue";

const emit = defineEmits<{
  "photo-click": [index: number];
}>();
</script>

<template>
  <PhotoGallery @photo-click="emit('photo-click', $event)" />
</template>
