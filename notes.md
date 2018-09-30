File Streamer
=============


HTTP server
-----------

### `POST /api/register`

takes no params at the moment `{}`

returns `{ id: "unique-generated-id" }`

### `POST /api/have`

takes a list of files we have (and our unique ID):

`{ id: "the-id", have: [ { fileId: "unique-per-file-id", path: "/some/path/to/myFile.mp3", size: 12341  }, ... ] }`

returns nothing `{}`

### `POST /api/ask`

takes the unique ID and max longpoll duration `{ id: "unique-generated-id", duration: 30000 }`

gives back details desribing what's currently needed. can hold connection open to limit polling.

`{ needs: [ { type: "data", fileId: "123abc", from: 0, to: 1024345 }, { type: "have" } ] }`

### `POST /api/data/:fileId`

Takes file data. Should contain headers or multipart params describing the "fileId", "from" and "to" and then the data itself. (Warp can handle this)

We can cache some amount of data in chunks of eg 1MB.

### `GET /api/data/:fileId`

Get some file! should support range requests.


State
-----

