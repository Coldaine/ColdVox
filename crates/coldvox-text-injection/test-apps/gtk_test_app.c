#include <gtk/gtk.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

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

    // Start the GTK main loop
    gtk_main();

    return 0;
}
