# The pslink library and binary

This is the pslink server part. It provides a webserver to be run behind another webserver like apache or nginx. Everything needed to run is bundled in the pslink binary. So you can compile everything locally and the copy the single binary to your server and run it.

Library features:
  * models for writing and retriving information from the database

Server/Binary features:
  * creation and migration of the database
  * creating an admin user
  * creating a `.env` file with all available options
  * launch the server
    * serve the wasm-file
    * serve styling and js bindings
    * provide a REST-JSON-Api to get and modify entries in the database
    * authentication