# Development

Depending on your selected options, your new workspace project contains a workspace member for each platform.
If you chose to develop with the router feature, each platform crate will have a `views` folder for your platform-specific views.
You are provided with a `ui` crate for shared UI and if you chose to use fullstack, you will have a `server` crate for your shared server functions.

### Serving Your App

Navigate to the platform crate of your choice:
```bash
cd web
```

and serve:

```bash
dx serve
```

