###
### Build stage
###
FROM oven/bun:1.1.24 AS base

WORKDIR /usr/src/app

COPY package.json bun.lock ./
RUN bun install --frozen-lockfile

COPY postcss.config.cjs svelte.config.js tailwind.config.js tsconfig.json vite.config.ts ./
COPY ./.svelte-kit/tsconfig.json ./.svelte-kit/tsconfig.json
COPY static ./static
COPY src ./src

RUN bun run build


###
### Release stage
###
FROM node:20-alpine AS release

WORKDIR /usr/src/app

COPY --from=base /usr/src/app/node_modules ./node_modules
COPY --from=base /usr/src/app/build ./build
COPY --from=base /usr/src/app/package.json ./

USER node

EXPOSE 3000

ENV HOST=0.0.0.0
ENV PUBLIC_API_ENDPOINT=http://localhost:9000
ENV ORIGIN=http://0.0.0.0

ENTRYPOINT ["node", "build", "--max-old-space-size=40"]
