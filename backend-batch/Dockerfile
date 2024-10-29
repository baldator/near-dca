FROM node:alpine

WORKDIR /app

COPY package*.json ./

RUN npm install

COPY *.js .

CMD [ "node", "index.js" ]