#include <gtk/gtk.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/stat.h>

// Callback function to handle text changes in the GtkEntry
static void on_text_changed(GtkEditable *editable, gpointer user_data) {
    const gchar *text = gtk_entry_get_text(GTK_ENTRY(editable));
    char filepath[256];
    snprintf(filepath, sizeof(filepath), "/tmp/coldvox_gtk_test_%d.txt", getpid());

    FILE *f = fopen(filepath, "w");
    if (f == NULL) {
        // In a real app, handle this error properly. For this test app, we'll just print.
        perror("Error opening file for writing");
        return;
    }
    fprintf(f, "%s", text);
    fclose(f);
}

// Create a ready file to signal that the app has started.
// This allows tests to detect when the app is ready without relying on text changes.
static gboolean create_ready_file(gpointer user_data) {
    char filepath[256];
    snprintf(filepath, sizeof(filepath), "/tmp/coldvox_gtk_test_%d.txt", getpid());

    // Securely create the file: O_CREAT | O_EXCL prevents clobbering or symlink attacks
    int fd = open(filepath, O_WRONLY | O_CREAT | O_EXCL, 0600);
    if (fd == -1) {
        perror("Failed to create ready file");
        return G_SOURCE_REMOVE; // Do not retry; tests will handle absence
    }

    // Write the PID to the file so tests can verify content is from the current process
    char pidbuf[32];
    int len = snprintf(pidbuf, sizeof(pidbuf), "%d", getpid());
    if (len > 0) {
        ssize_t w = write(fd, pidbuf, (size_t)len);
        if (w < 0) {
            perror("Failed to write PID to ready file");
        }
    }

    close(fd);
    return G_SOURCE_REMOVE; // Run once
}

int main(int argc, char *argv[]) {
    gtk_init(&argc, &argv);

    // Create the main window
    GtkWidget *window = gtk_window_new(GTK_WINDOW_TOPLEVEL);
    gtk_window_set_title(GTK_WINDOW(window), "GTK Test App");
    gtk_window_set_default_size(GTK_WINDOW(window), 200, 50);
    g_signal_connect(window, "destroy", G_CALLBACK(gtk_main_quit), NULL);

    // Create a text entry widget
    GtkWidget *entry = gtk_entry_new();
    gtk_container_add(GTK_CONTAINER(window), entry);

    // Connect the "changed" signal to our callback
    // The "changed" signal is emitted for every character change.
    g_signal_connect(G_OBJECT(entry), "changed", G_CALLBACK(on_text_changed), NULL);

    // Show all widgets
    gtk_widget_show_all(window);

    // Ensure the entry widget has focus when the window appears
    gtk_widget_grab_focus(entry);

    // Schedule ready file creation for when the main loop starts
    g_idle_add(create_ready_file, NULL);

    // Start the GTK main loop
    gtk_main();

    return 0;
}
